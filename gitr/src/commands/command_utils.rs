use crate::{
    commands::commands_fn,
    diff::Diff,
    file_manager::{
        self, get_commit, get_current_commit, get_current_repo, get_head, read_index,
        update_working_directory, visit_dirs, get_tags,
    },
    git_transport::ref_discovery::read_long_stream,
};
use crate::{
    file_manager::get_branches,
    git_transport::{pack_file::PackFile, ref_discovery},
    objects::git_object::GitObject,
};
use crate::{
    gitr_errors::GitrError,
    objects::{
        blob::{Blob, TreeEntry},
        commit::Commit,
        tag::Tag,
        tree::Tree,
    },
};
use flate2::write::ZlibEncoder;
use flate2::Compression;

use sha1::{Digest, Sha1};
use std::{
    collections::{HashMap, HashSet},
    fs::{self},
    io::{self, Read, Write},
    net::TcpStream,
    path::Path, process::{Command, Stdio},
};

/***************************
 ***************************
 *  DEFLATING AND HASHING
 **************************
 **************************/

/// compression function for Vec<u8>
pub fn flate2compress2(input: Vec<u8>) -> Result<Vec<u8>, GitrError> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    match encoder.write_all(&input) {
        Ok(_) => {}
        Err(_) => return Err(GitrError::CompressionError),
    };
    let compressed_bytes = match encoder.finish() {
        Ok(bytes) => bytes,
        Err(_) => return Err(GitrError::CompressionError),
    };
    Ok(compressed_bytes)
}
/// hashing function for Vec<u8>
pub fn sha1hashing2(input: Vec<u8>) -> Vec<u8> {
    let mut hasher = Sha1::new();
    hasher.update(&input);
    let result = hasher.finalize();
    result.to_vec()
}
/// hashing function for String
pub fn sha1hashing(input: String) -> Vec<u8> {
    let mut hasher = Sha1::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    result.to_vec()
}
/// compression function for String
pub fn flate2compress(input: String) -> Result<Vec<u8>, GitrError> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    match encoder.write_all(input.as_bytes()) {
        Ok(_) => {}
        Err(_) => return Err(GitrError::CompressionError),
    };
    let compressed_bytes = match encoder.finish() {
        Ok(bytes) => bytes,
        Err(_) => return Err(GitrError::CompressionError),
    };
    Ok(compressed_bytes)
}

/***************************
 ***************************
 * CAT-FILE FUNCTIONS
 **************************
 **************************/

// obtain the hash of an object
pub fn get_object_hash(
    cliente: String,
    file_path: &mut String,
    write: bool,
) -> Result<String, GitrError> {
    *file_path = file_manager::get_current_repo(cliente.clone())?.to_string() + "/" + file_path;
    let raw_data = file_manager::read_file(file_path.to_string())?;
    let blob = Blob::new(raw_data)?;
    let res: String = blob.get_hash();
    if write {
        blob.save(cliente)?;
    }
    Ok(res)
}

/// returns object hash, output, size and type
pub fn get_object_properties(
    flags: Vec<String>,
    cliente: String,
) -> Result<(String, String, String, String), GitrError> {
    let object_hash = &flags[1];
    let res_output = file_manager::read_object(
        object_hash,
        file_manager::get_current_repo(cliente.clone())?,
        true,
    )?;
    let object_type = res_output.split(' ').collect::<Vec<&str>>()[0];
    let _size = res_output.split(' ').collect::<Vec<&str>>()[1];
    let size = _size.split('\0').collect::<Vec<&str>>()[0];
    Ok((
        object_hash.to_string(),
        res_output.clone(),
        size.to_string(),
        object_type.to_string(),
    ))
}

//Output the contents or other properties such as size, type or delta information of an object
pub fn _cat_file(flags: Vec<String>, cliente: String) -> Result<String, GitrError> {
    let (object_hash, res_output, size, object_type) =
        get_object_properties(flags.clone(), cliente)?;
    let data_requested = &flags[0];
    if data_requested == "-t" {
        return Ok(object_type);
    }
    if data_requested == "-s" {
        return Ok(size);
    }
    if data_requested == "-p" {
        let raw_data = match res_output.split_once('\0') {
            Some((_object_type, raw_data)) => raw_data,
            None => {
                println!("Error: invalid object type");
                return Err(GitrError::FileReadError(object_hash.to_string()));
            }
        };
        match object_type.as_str() {
            "blob" => Ok(raw_data.to_string()),
            "tree" => Ok(get_tree_data(raw_data)),
            "commit" => Ok(raw_data.to_string()),
            "tag" => Ok(raw_data.to_string()),
            _ => Err(GitrError::FileReadError(object_hash.to_string())),
        }
    } else {
        Ok("Invalid option. Expected <[-t/-s/-p>".to_string())
    }
}

/***************************
 ***************************
 *  OBJECT PRINTS
 **************************
 **************************/

pub fn print_blob_data(raw_data: &str) {
    println!("{}", raw_data);
}

// receives the raw data and returns the tree data
pub fn get_tree_data(raw_data: &str) -> String {
    let files = raw_data.split('\n').collect::<Vec<&str>>();
    let mut tree_data = String::new();
    for object in files {
        let file_atributes = object.split(' ').collect::<Vec<&str>>();
        let file_mode = file_atributes[0];
        let file_path_hash = file_atributes[1];
        let file_path = file_path_hash.split('\0').collect::<Vec<&str>>()[0];
        let file_hash = file_path_hash.split('\0').collect::<Vec<&str>>()[1];
        let file_type = if file_mode == "100644" {
            "blob"
        } else {
            "tree"
        };

        let entry = format!("{} {} {} {}\n", file_mode, file_type, file_hash, file_path);
        tree_data.push_str(entry.as_str());
    }
    tree_data
}
pub fn print_commit_data(raw_data: &str) {
    println!("{}", raw_data);
}

pub fn print_tag_data(raw_data: &str) {
    println!("{}", raw_data);
}

/***************************
 ***************************
 *  CHECKOUT FUNCTIONS
 **************************
 **************************/

/// create a tree (and blobs inside it) for checkout function
pub fn create_trees(
    tree_map: HashMap<String, Vec<String>>,
    current_dir: String,
    cliente: String,
) -> Result<Tree, GitrError> {
    let mut tree_entry: Vec<(String, TreeEntry)> = Vec::new();
    if let Some(objs) = tree_map.get(&current_dir) {
        for obj in objs {
            if tree_map.contains_key(obj) {
                let new_tree = create_trees(tree_map.clone(), obj.to_string(), cliente.clone())?;
                tree_entry.push((obj.clone(), TreeEntry::Tree(new_tree)));
            } else {
                let raw_data = file_manager::read_file(obj.clone())?;
                let blob = Blob::new(raw_data)?;
                tree_entry.push((obj.clone(), TreeEntry::Blob(blob)));
            }
        }
    };
    let tree = Tree::new(tree_entry)?;
    tree.save(cliente)?;
    Ok(tree)
}

/// writes the main tree for a commit, then writes the commit and the branch if necessary
pub fn get_tree_entries(
    message: String,
    second_parent: String,
    cliente: String,
) -> Result<(), GitrError> {
    let (tree_map, tree_order) = get_hashmap_for_checkout(cliente.clone())?;
    let final_tree = create_trees(tree_map, tree_order[0].clone(), cliente.clone())?;
    final_tree.save(cliente.clone())?;
    write_new_commit_and_branch(final_tree, message, second_parent, cliente)?;
    Ok(())
}
/// write a new commit and the branch if necessary
pub fn write_new_commit_and_branch(
    final_tree: Tree,
    message: String,
    second_parent: String,
    cliente: String,
) -> Result<(), GitrError> {
    let head = file_manager::get_head(cliente.clone())?;
    let repo = file_manager::get_current_repo(cliente.clone())?;
    let path_complete = repo.clone() + "/gitr/" + head.as_str();
    if fs::metadata(path_complete.clone()).is_err() {
        let dir = repo + "/gitr/refs/heads/master";
        file_manager::write_file(path_complete, final_tree.get_hash())?;
        if !Path::new(&dir).exists() {
            let current_commit = file_manager::get_current_commit(cliente.clone())?;
            file_manager::write_file(dir.clone(), current_commit)?;
        }
        let commit = Commit::new(
            final_tree.get_hash(),
            vec!["None".to_string()],
            cliente.clone(),
            cliente.clone(),
            message,
            cliente.clone(),
        )?;
        commit.save(cliente.clone())?;
        file_manager::write_file(dir, commit.get_hash())?;
    } else {
        let dir = repo + "/gitr/" + &head;
        let current_commit = file_manager::get_current_commit(cliente.clone())?;
        let mut parents = vec![current_commit];
        if second_parent != "None" {
            parents.push(second_parent);
        }
        let commit = Commit::new(
            final_tree.get_hash(),
            parents,
            cliente.clone(),
            cliente.clone(),
            message,
            cliente.clone(),
        )?;
        commit.save(cliente)?;
        file_manager::write_file(dir, commit.get_hash())?;
    }
    Ok(())
}

/// returns a hashmap to create trees (using the index)
type CheckoutHashMap = (HashMap<String, Vec<String>>, Vec<String>);

pub fn get_hashmap_for_checkout(cliente: String) -> Result<CheckoutHashMap, GitrError> {
    let mut tree_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut tree_order: Vec<String> = Vec::new();
    let index_files = read_index(cliente.clone())?;
    for file_info in index_files.split('\n') {
        let file_path = file_info.split(' ').collect::<Vec<&str>>()[3];
        let splitted_file_path = file_path.split('/').collect::<Vec<&str>>();
        for (i, dir) in (splitted_file_path.clone()).iter().enumerate() {
            if let Some(last_element) = splitted_file_path.last() {
                if dir == last_element {
                    update_hashmap_tree_entry(
                        &mut tree_map,
                        splitted_file_path[i - 1],
                        file_path.to_string(),
                    );
                } else {
                    if !tree_map.contains_key(dir as &str) && (dir != &cliente) {
                        tree_map.insert(dir.to_string(), vec![]);
                        tree_order.push(dir.to_string());
                    }
                    if i == 0 {
                        continue;
                    }
                    update_hashmap_tree_entry(
                        &mut tree_map,
                        splitted_file_path[i - 1],
                        dir.to_string(),
                    );
                }
            }
        }
    }
    Ok((tree_map, tree_order))
}

/// update the tree entries hashmap
pub fn update_hashmap_tree_entry(
    tree_map: &mut HashMap<String, Vec<String>>,
    previous_dir: &str,
    file_path: String,
) {
    if tree_map.contains_key(previous_dir) {
        match tree_map.get_mut(previous_dir) {
            Some(folder) => {
                if !folder.contains(&file_path.to_string()) {
                    folder.push(file_path.to_string());
                }
            }
            None => {
                println!("No se encontro el folder");
            }
        }
    }
}

// check if the branch exists and if its -b used it creates a branch
pub fn get_branch_to_checkout(
    args_received: Vec<String>,
    cliente: String,
) -> Result<String, GitrError> {
    let mut branch_to_checkout: String = args_received[0].clone();
    if args_received.len() == 2 && args_received[0] == "-b" {
        branch_to_checkout = args_received[1].clone();
        branch_newbranch_flag(branch_to_checkout.clone(), cliente.clone())?;
    }
    if !branch_exists(branch_to_checkout.clone(), cliente.clone()) {
        return Err(GitrError::BranchNonExistsError(args_received[0].clone()));
    }
    Ok(branch_to_checkout)
}

/***************************
 ***************************
 *    GET USER DATA
 **************************
 **************************/

/// returns the username
pub fn get_current_username(cliente: String) -> String {
    cliente
}

/// returns the mail from config
pub fn get_user_mail_from_config(cliente: String) -> Result<String, GitrError> {
    let config_data = match file_manager::read_file(cliente + "/gitrconfig") {
        Ok(config_data) => config_data,
        Err(e) => return Err(GitrError::FileReadError(e.to_string())),
    };

    let lines = config_data.split('\n').collect::<Vec<&str>>();
    let email = lines[1].split('=').collect::<Vec<&str>>()[1].trim_start();
    Ok(email.to_string())
}

/***************************
 ***************************
 *   LS-FILES FUNCTIONS
 **************************
 **************************/

/// returns cached ls-files
pub fn get_ls_files_cached(cliente: String) -> Result<String, GitrError> {
    let mut string_res = String::new();
    let index = match read_index(cliente.clone()) {
        Ok(index) => index,
        Err(_) => return Ok(string_res),
    };
    for file_path in index.lines() {
        let correct_path = match file_path.split_once('/') {
            Some((_path, file)) => file,
            None => file_path,
        };
        let line = correct_path.to_string() + "\n";
        string_res.push_str(&line);
    }
    Ok(string_res)
}

/// returns deleted files or modified files depending on bool received
pub fn get_ls_files_deleted_modified(deleted: bool, cliente: String) -> Result<String, GitrError> {
    let mut res = String::new();
    let (not_staged, _, _) = get_untracked_notstaged_files(cliente.clone())?;
    let files_not_staged = get_status_files_not_staged(&not_staged, cliente.clone())?;
    for line in files_not_staged.lines() {
        if line.contains("deleted") && deleted {
            let line = line.replace("deleted:   ", "");
            res.push_str(&(line + "\n"));
        } else if !deleted && (line.contains("modified") || line.contains("deleted")) {
            let mut line = line.replace("modified   ", "");
            line = line.replace("deleted:   ", "");
            res.push_str(&(line + "\n"));
        }
    }
    Ok(res)
}

/***************************
 ***************************
 *   BRANCH FUNCTIONS
 **************************
 **************************/

/// print all the branches in repo
pub fn print_branches(cliente: String) -> Result<String, GitrError> {
    let mut res = String::new();
    let head = file_manager::get_head(cliente.clone())?;
    let head_vec = head.split('/').collect::<Vec<&str>>();
    let head = head_vec[head_vec.len() - 1];
    let branches = file_manager::get_branches(cliente.clone())?;
    for branch in branches {
        if head == branch {
            let index_branch = format!("* \x1b[92m{}\x1b[0m", branch);
            res.push_str(&(index_branch + "\n"));
            continue;
        }
        res.push_str(&(format!("{}\n", branch)));
    }
    Ok(res)
}

/// check if a branch exists
pub fn branch_exists(branch: String, cliente: String) -> bool {
    let branches = file_manager::get_branches(cliente.clone());
    let branches = match branches {
        Ok(branches) => branches,
        Err(_) => return false,
    };
    for branch_name in branches {
        if branch_name == branch {
            return true;
        }
    }
    false
}

/// branch -d flag function
pub fn branch_delete_flag(branch: String, cliente: String) -> Result<(), GitrError> {
    if !branch_exists(branch.clone(), cliente.clone()) {
        return Err(GitrError::BranchNonExistsError(branch));
    }
    file_manager::delete_branch(branch, false, cliente.clone())?;
    Ok(())
}

/// branch -m flag function
pub fn branch_move_flag(
    branch_origin: String,
    branch_destination: String,
    cliente: String,
) -> Result<(), GitrError> {
    if !branch_exists(branch_origin.clone(), cliente.clone()) {
        return Err(GitrError::BranchNonExistsError(branch_origin));
    }
    if branch_exists(branch_destination.clone(), cliente.clone()) {
        return Err(GitrError::BranchAlreadyExistsError(branch_destination));
    }
    let repo = get_current_repo(cliente.clone())?;
    let old_path = format!("{}/gitr/refs/heads/{}", repo.clone(), branch_origin);
    let new_path = format!("{}/gitr/refs/heads/{}", repo.clone(), branch_destination);
    file_manager::move_branch(old_path.clone(), new_path.clone())?;
    let head = get_head(cliente.clone())?;
    if branch_origin == head.split('/').collect::<Vec<&str>>()[2] {
        let ref_correct = format!("refs/heads/{}", branch_destination);
        file_manager::update_head(&ref_correct, cliente.clone())?;
    }
    Ok(())
}

/// branch <newbranch> flag function
pub fn branch_newbranch_flag(branch: String, cliente: String) -> Result<(), GitrError> {
    let repo = get_current_repo(cliente.clone())?;
    if branch_exists(branch.clone(), cliente.clone()) {
        return Err(GitrError::BranchAlreadyExistsError(branch));
    }
    let current_commit = file_manager::get_current_commit(cliente.clone())?;
    file_manager::write_file(
        format!("{}/gitr/refs/heads/{}", repo.clone(), branch),
        current_commit,
    )?;
    Ok(())
}

/// receives a branch_name and returns the commit hash
pub fn branch_commits_list(branch_name: String, cliente: String) -> Result<Vec<String>, GitrError> {
    let mut commits = Vec::new();

    let mut commit = file_manager::get_commit(branch_name, cliente.clone())?;
    
    commits.push(commit.clone());
    loop {
        let parent = file_manager::get_parent_commit(commit.clone(), cliente.clone())?[0].clone();

        if parent == "None" {
            break;
        }

        commit = parent;
        commits.push(commit.clone());
    }
    Ok(commits)
}
/***************************
 ***************************
 *   COMMIT FUNCTIONS
 **************************
 **************************/

/// prints the commit confirmation after commiting
pub fn print_commit_confirmation(message: String, cliente: String) -> Result<(), GitrError> {
    let branch = get_head(cliente.clone())?.split('/').collect::<Vec<&str>>()[2].to_string();
    let hash_recortado = &get_current_commit(cliente.clone())?[0..7];
    println!("[{} {}] {}", branch, hash_recortado, message);
    Ok(())
}
/// check if a commit exist
pub fn commit_existing(cliente: String) -> Result<(), GitrError> {
    let repo = file_manager::get_current_repo(cliente.clone())?;
    let head = file_manager::get_head(cliente.clone())?;
    let branch_name = head.split('/').collect::<Vec<&str>>()[2];
    if fs::metadata(repo.clone() + "/gitr/" + &head).is_err() {
        return Err(GitrError::NoCommitExisting(branch_name.to_string()));
    }
    Ok(())
}

/***************************
 ***************************
 *   MERGE FUNCTIONS
 **************************
 **************************/

/// receives a branch name and makes fast forward merge
pub fn fast_forward_merge(branch_name: String, cliente: String) -> Result<(), GitrError> {
    let commit: String = file_manager::get_commit(branch_name, cliente.clone())?;
    let head = get_head(cliente.clone())?;
    let repo = get_current_repo(cliente.clone())?;
    let mut path = format!("{}/gitr/{}", repo, head);

    if cliente.contains('/') {
        path = path.replace("/gitr/", "/");
    }

    file_manager::write_file(path, commit.clone())?;
    
    if cliente.contains('/') {
        return Ok(());
    }
    update_working_directory(commit, cliente.clone())?;
    Ok(())
}

pub fn get_blobs_from_commit(commit_hash: String, cliente: String) -> Result<(), GitrError> {
    let _path_and_hash_hashmap = get_commit_hashmap(commit_hash, cliente.clone())?;
    Ok(())
}

fn _aplicar_diffs(string_archivo: String, diff: Diff) -> Result<Vec<String>, GitrError> {
    let mut archivo_reconstruido = vec![];

    let mut j = 0; //con este indexo el diff
    let max_j = diff.lineas.len();

    for (i, line) in string_archivo.lines().enumerate() {
        if j < max_j {
            if diff.lineas[j].0 == i {
                //en la linea hay una operación
                if !diff.lineas[j].1 {
                    //es un delete

                    if j + 1 >= max_j {
                        j += 1;
                        continue;
                    }

                    if diff.lineas[j + 1].1 {
                        //hay un add tambien
                        archivo_reconstruido.push(diff.lineas[j + 1].2.clone() + "\n"); //pusheo el add, ignorando lo que se borró
                        j += 2;
                    } else {
                        //solo delete, no pusheo
                        j += 1;
                    }
                    continue;
                } else {
                    //no hay delete, solo add
                    archivo_reconstruido.push(diff.lineas[j].2.clone() + "\n"); //pusheo el add
                    archivo_reconstruido.push(line.to_string() + "\n"); //pusheo la linea del base
                    j += 1;
                }
            } else {
                //si no hay operacion, pusheo la linea del base y sigo
                archivo_reconstruido.push(line.to_string() + "\n");
            }
        } else {
            //si no hay operacion, pusheo la linea del base y sigo
            archivo_reconstruido.push(line.to_string() + "\n");
        }
    }

    for i in j..diff.lineas.len() {
        //agrego los diffs que me faltaron antes
        archivo_reconstruido.push(diff.lineas[i].2.clone() + "\n");
    }
    Ok(archivo_reconstruido)
}

fn aplicar_diffs(path: String, diff: Diff) -> Result<(), GitrError> {
    let string_archivo = file_manager::read_file(path.clone())?;
    let archivo_reconstruido = _aplicar_diffs(string_archivo.clone(), diff.clone())?;
    file_manager::write_file(path, archivo_reconstruido.concat().to_string())?;

    Ok(()) 
}

fn armar_conflict2(origin_conflicts: String, new_conflicts: String) -> String {
    //armo el conflict y vacío los vectores para "reiniciarlos"
    let conflict = [
        "<<<<<<< HEAD\n",
        origin_conflicts.as_str(),
        "\n=======\n",
        new_conflicts.as_str(),
        "\n>>>>>>> BRANCH",
    ]
    .concat();
    conflict
}

fn juntar_consecutivos(diff: Diff) -> Diff {
    let mut diff_juntado = Diff::new("".to_string(), "".to_string());

    let input = diff.lineas.clone();
    let mut output = Vec::new();
    let mut corrimiento = 1;
    let mut corrimiento_total = 0;
    for (i, (index, accion, s)) in input.iter().enumerate() {
        if !accion {
            continue;
        }

        if *index == 0 || i == 0 {
            output.push((*index, *accion, s.to_string()));
        } else if let Some((prev_num, _, prev_str)) = output.last_mut() {
            if *prev_num + corrimiento == *index {
                prev_str.push_str(("\n".to_string() + s.as_str()).as_str());
                corrimiento += 1;
                corrimiento_total += 1;
            } else {
                output.push((*index, *accion, s.to_string()));
                corrimiento = 1;
            }
        } else {
            output.push((*index, *accion, s.to_string()));
        }
    }
    for (index, accion, s) in input.iter() {
        if !accion {
            output.push((*index, *accion, s.to_string()));
            continue;
        }
    }
    output.sort_by(|a, b| {
        let cmp_first = a.0.cmp(&b.0);
        let cmp_second = a.1.cmp(&b.1);

        if cmp_first == std::cmp::Ordering::Equal && cmp_second == std::cmp::Ordering::Equal {
            std::cmp::Ordering::Equal
        } else if cmp_first == std::cmp::Ordering::Equal {
            cmp_second
        } else {
            cmp_first.then(cmp_second)
        }
    });

    diff_juntado.lineas = output
        .into_iter()
        .map(|(index, accion, s)| (index, accion, s))
        .collect();
    diff_juntado.lineas_extra = corrimiento_total;
    diff_juntado
}

fn preparar_lineas_para_comparar(
    diff_base_origin: Diff,
    diff_base_branch: Diff,
) -> Vec<(usize, bool, String, String)> {
    let origin_consec = juntar_consecutivos(diff_base_origin).lineas;
    let new_consec = juntar_consecutivos(diff_base_branch).lineas;

    let mut origin_tagged: Vec<(usize, bool, String, String)> = Vec::new();
    let mut new_tagged = Vec::new();

    for (i, accion, linea) in origin_consec.clone() {
        origin_tagged.push((i, accion, linea, "origin".to_string()));
    }
    for (i, accion, linea) in new_consec.clone() {
        new_tagged.push((i, accion, linea, "new".to_string()));
    }

    let mut joined_diffs = origin_tagged;
    joined_diffs.extend(new_tagged);

    let mut vistos = HashSet::new();
    let mut unicos = Vec::new();

    for tupla in joined_diffs {
        let key = (tupla.0, tupla.1, tupla.2.clone());

        if vistos.insert(key.clone()) {
            unicos.push(tupla);
        }
    }

    let mut result = unicos.clone();

    result.sort_by(|a, b| {
        let cmp_first = a.0.cmp(&b.0);
        let cmp_second = a.1.cmp(&b.1);

        if cmp_first == std::cmp::Ordering::Equal && cmp_second == std::cmp::Ordering::Equal {
            std::cmp::Ordering::Equal
        } else if cmp_first == std::cmp::Ordering::Equal {
            cmp_second
        } else {
            cmp_first.then(cmp_second)
        }
    });
    result
}

fn comparar_diffs(
    diff_base_origin: Diff,
    diff_base_branch: Diff,
    limite_archivo: usize,
) -> Result<(Diff, bool), GitrError> {
    let mut diff_final = Diff::new("".to_string(), "".to_string());

    let result = preparar_lineas_para_comparar(diff_base_origin, diff_base_branch);

    // dic solo con los de agregar
    let mut map: HashMap<usize, Vec<(String, String)>> = HashMap::new();
    for (index, flag, string, tag) in result.clone() {
        if flag {
            map.entry(index)
                .or_default()
                .push((string, tag.to_string()));
        }
    }

    let mut indices_ya_visitados = HashSet::new();

    let mut hay_extra = false;
    let mut iter_count: i8 = -1;
    let mut hubo_conflict = false;
    for (index, flag, string, _) in result.clone() {
        iter_count += 1;

        if index > limite_archivo {
            hay_extra = true;
            break;
        }

        if indices_ya_visitados.contains(&index) {
            continue;
        }

        if !flag {
            //si es delete, pusheo porque no van a haber conflicts de delete.
            diff_final.lineas.push((index, flag, string));
            continue;
        }

        let lineas = map.get(&index).unwrap(); //entra al diccionario y se trae una linea o varias si hay conflict

        if lineas.len() == 1 {
            //si cuando me traigo las lineas, traigo una sola, es porque no hay dos operaciones de add en la misma linea.
            diff_final.lineas.push((index, flag, lineas[0].0.clone()));
            indices_ya_visitados.insert(index);

            continue;
        }

        //para este punto hay un conflict
        hubo_conflict = true;
        let conflict = armar_conflict2(lineas[0].0.clone(), lineas[1].0.clone());
        diff_final.lineas.push((index, flag, conflict));
        indices_ya_visitados.insert(index);
    }
    if hay_extra {
        if result[iter_count as usize] == result[result.len() - 1] {
            /*
            lo que quiero ver es si tengo una sola linea de diff por fuera del archivo
            (y en ese saco solamente la pusheo y listo)

            o si tengo 2 lineas, y en ese caso tengo que armar el conflict y pushear

            solo puedo tener esos 2 casos en este punto
             */
            diff_final.lineas.push((
                result[iter_count as usize].0,
                result[iter_count as usize].1,
                result[iter_count as usize].2.clone(),
            ));
        } else {
            let mut origin = result[iter_count as usize].clone();
            let mut new = result[iter_count as usize + 1].clone();
            if new.3 == "origin" {
                origin = result[iter_count as usize + 1].clone();
                new = result[iter_count as usize].clone();
            }

            let conflict = armar_conflict2(origin.2, new.2);
            diff_final.lineas.push((
                result[iter_count as usize].0,
                result[iter_count as usize].1,
                conflict,
            ));
        }
    }

    Ok((diff_final, hubo_conflict))
}

pub fn three_way_merge(
    base_commit: String,
    origin_commit: String,
    branch_commit: String,
    cliente: String,
) -> Result<(bool, Vec<String>), GitrError> {
    let branch_hashmap = get_commit_hashmap(branch_commit.clone(), cliente.clone())?;
    let mut origin_hashmap = get_commit_hashmap(origin_commit.clone(), cliente.clone())?;
    file_manager::add_new_files_from_merge(
        origin_hashmap.clone(),
        branch_hashmap.clone(),
        cliente.clone(),
    )?;
    origin_hashmap = get_commit_hashmap(origin_commit.clone(), cliente.clone())?;
    let base_hashmap = get_commit_hashmap(base_commit.clone(), cliente.clone())?;
    let mut hubo_conflict = false;
    let mut archivos_conflict = Vec::new();


    for (path, origin_file_hash) in origin_hashmap.iter() {
        let origin_file_data: String =
            file_manager::read_file_data_from_blob_hash(origin_file_hash.clone(), cliente.clone())?;
        if branch_hashmap.contains_key(&path.clone()) {
            let branch_file_hash = branch_hashmap[path].clone(); //aax
            let branch_file_data = file_manager::read_file_data_from_blob_hash(
                branch_file_hash.clone(),
                cliente.clone(),
            )?;

            if origin_file_hash == &branch_file_hash {
                continue;
            }

            let mut base_file_hash = "".to_string();
            let base_file_data;

            if !base_hashmap.contains_key(path) {
                base_file_data = "".to_string();
            } else {
                base_file_hash = base_hashmap[path].clone(); // chequear que capaz puede no exisiir en base
                base_file_data = file_manager::read_file_data_from_blob_hash(
                    base_file_hash.clone(),
                    cliente.clone(),
                )?;
            }

            if &base_file_hash == origin_file_hash {
                let diff_base_branch = Diff::new(base_file_data, branch_file_data);
                if cliente.contains('/') {
                    _aplicar_diffs(origin_file_data.clone(), diff_base_branch)?;
                } else {
                    aplicar_diffs(path.clone(), diff_base_branch)?;
                }
                continue;
            }

            if base_file_hash == branch_file_hash {
                continue;
            }

            let mut len_archivo = base_file_data.len();
            if len_archivo == 0 {
                if branch_file_data.len() > origin_file_data.len() {
                    len_archivo = branch_file_data.len();
                } else {
                    len_archivo = origin_file_data.len();
                }
            }

            let diff_base_origin = Diff::new(base_file_data.clone(), origin_file_data.clone());
            let diff_base_branch = Diff::new(base_file_data.clone(), branch_file_data.clone());
            let union_diffs;

            (union_diffs, hubo_conflict) =
                comparar_diffs(diff_base_origin, diff_base_branch, len_archivo - 1)?; //une los diffs o da el conflict

            if hubo_conflict {
                if let Some(nombre_archivo) = path.split('/').last() {
                    archivos_conflict.push(nombre_archivo.to_string().clone());
                }
            }

            if base_file_data.is_empty() || base_file_data.is_empty() {
                let archivo_reconstruido = _aplicar_diffs("".to_string(), union_diffs)?;
                file_manager::write_file(
                    path.to_string() + "_mergeado",
                    archivo_reconstruido.concat().to_string(),
                )?;
            } else if cliente.contains('/') {
                _aplicar_diffs(origin_file_data.clone(), union_diffs)?;
            } else {
                aplicar_diffs(path.clone(), union_diffs)?;
            }
        } else {
            continue;
        }
    }

    Ok((hubo_conflict, archivos_conflict))
}

pub fn create_merge_commit(
    branch_name: String,
    branch_commit: String,
    cliente: String,
) -> Result<(), GitrError> {
    let index_path = file_manager::get_current_repo(cliente.clone())?.to_string() + "/gitr/index";
    if !Path::new(&index_path).exists() {
        return commands_fn::status(vec![], cliente.clone());
    }

    let message = format!("Merge branch '{}'", branch_name);
    get_tree_entries(message.to_string(), branch_commit, cliente.clone())?;
    print_commit_confirmation(message, cliente.clone())?;
    Ok(())
}

/***************************
 ***************************
 *   STATUS FUNCTIONS
 **************************
 **************************/

pub fn get_status(cliente: String) -> Result<String, GitrError> {
    let mut res = String::new();
    res.push_str(&(status_print_current_branch(cliente.clone())? + "\n"));
    let (not_staged, untracked_files, hayindex) = get_untracked_notstaged_files(cliente.clone())?;
    let (new_files, modified_files) = get_tobe_commited_files(&not_staged, cliente.clone())?;
    res.push_str(&get_status_files_to_be_comited(
        &new_files,
        &modified_files,
    )?);
    res.push_str(&get_status_files_not_staged(&not_staged, cliente.clone())?);
    res.push_str(&get_status_files_untracked(&untracked_files, hayindex));
    if new_files.is_empty()
        && modified_files.is_empty()
        && not_staged.is_empty()
        && untracked_files.is_empty()
    {
        res.push_str("nothing to commit, working tree clean\n");
    }
    Ok(res)
}
pub fn get_working_dir_hashmap(cliente: String) -> Result<HashMap<String, String>, GitrError> {
    let mut working_dir_hashmap = HashMap::new();
    let repo = file_manager::get_current_repo(cliente.clone())?;

    let path = Path::new(&repo);
    let files = visit_dirs(path);
    for file_path in files {
        let file_data = file_manager::read_file(file_path.clone())?;

        let blob = Blob::new(file_data.clone())?;
        let hash = blob.get_hash();
        working_dir_hashmap.insert(file_path, hash);
    }
    Ok(working_dir_hashmap)
}

pub fn get_status_files_to_be_comited(
    new_files: &Vec<String>,
    modified_files: &Vec<String>,
) -> Result<String, GitrError> {
    let mut res = String::new();
    if !new_files.is_empty() || !modified_files.is_empty() {
        let header1 = "Changes to be committed:\n".to_string();
        let header2 = "  (use \"rm <file>...\" to unstage)\n".to_string();
        res.push_str(&header1);
        res.push_str(&header2);
        for file in new_files.clone() {
            let file_name = match file.split_once('/') {
                Some((_path, file)) => file.to_string(),
                None => file.to_string(),
            };
            let line = format!("\t\x1b[92mnew file:   {}\x1b[0m\n", file_name);
            res.push_str(&line);
        }
        for file in modified_files.clone() {
            let file_name = match file.split_once('/') {
                Some((_path, file)) => file.to_string(),
                None => file.to_string(),
            };
            let line = format!("\t\x1b[92mmodified:   {}\x1b[0m\n", file_name);
            res.push_str(&line);
        }
    }
    Ok(res)
}

pub fn get_status_files_not_staged(
    not_staged: &Vec<String>,
    cliente: String,
) -> Result<String, GitrError> {
    let mut res = String::new();
    let (index, hayindex) = get_index_hashmap(cliente.clone())?;
    let working_dir_hashmap = get_working_dir_hashmap(cliente.clone())?;
    if !not_staged.is_empty() {
        let header1 = "Changes not staged for commit:\n".to_string();
        let header2 = "  (use \"add <file>...\" to update what will be committed)\n".to_string();
        let header3 =
            "  (use \"rm <file>...\" to discard changes in working directory)\n".to_string();
        res.push_str(&header1);
        res.push_str(&header2);
        res.push_str(&header3);
        for file in not_staged.clone() {
            let file_name = match file.clone().split_once('/') {
                Some((_path, file)) => file.to_string(),
                None => file.clone(),
            };
            if hayindex
                && index.contains_key(&file)
                && !working_dir_hashmap.contains_key(file.as_str())
            {
                let line = format!("\t\x1b[31mdeleted:   {}\x1b[0m\n", file_name);
                res.push_str(&line);
            } else {
                let line = format!("\t\x1b[92mmodified   {}\x1b[0m\n", file_name);
                res.push_str(&line);
            }
        }
    }
    Ok(res)
}

pub fn get_status_files_untracked(untracked_files: &Vec<String>, hayindex: bool) -> String {
    let mut res = String::new();
    if !untracked_files.is_empty() {
        let header1 = "Untracked files:\n".to_string();
        let header2 =
            "  (use \"add <file>...\" to include in what will be committed)\n".to_string();
        res.push_str(&header1);
        res.push_str(&header2);
        for file in untracked_files.clone() {
            let file_name = match file.split_once('/') {
                Some((_path, file)) => file.to_string(),
                None => file,
            };
            let output = format!("\t\x1b[31m{}\x1b[0m\n", file_name);
            res.push_str(&output)
        }

        if !hayindex {
            let nothing_output =
                "nothing added to commit but untracked files present (use \"add\" to track)\n"
                    .to_string();
            res.push_str(&nothing_output);
        }
    }
    res
}

pub fn status_print_current_branch(cliente: String) -> Result<String, GitrError> {
    let mut res = String::new();
    let head = file_manager::get_head(cliente.clone())?;
    let current_branch = head.split('/').collect::<Vec<&str>>()[2];
    res.push_str(&format!("On branch {}\n\n", current_branch));
    if commit_existing(cliente).is_err() {
        res.push_str("No commits yet\n");
    }
    Ok(res)
}

pub fn get_index_hashmap(cliente: String) -> Result<(HashMap<String, String>, bool), GitrError> {
    let mut index_hashmap = HashMap::new();
    let mut hayindex = true;
    let index_data = match file_manager::read_index(cliente.clone()) {
        Ok(data) => data,
        Err(_) => {
            hayindex = false;
            String::new()
        }
    };
    if hayindex {
        for index_entry in index_data.split('\n') {
            let attributes = index_entry.split(' ').collect::<Vec<&str>>();
            let path = attributes[3].to_string();
            let hash = attributes[1].to_string();
            index_hashmap.insert(path, hash);
        }
    }
    Ok((index_hashmap, hayindex))
}

pub fn get_subtrees_data(
    hash_of_tree_to_read: String,
    file_path: String,
    tree_hashmap: &mut HashMap<String, String>,
    cliente: String,
) -> Result<(), GitrError> {
    let tree_data = file_manager::read_object(
        &hash_of_tree_to_read,
        file_manager::get_current_repo(cliente.clone())?,
        true,
    )?;

    let tree_entries = match tree_data.split_once('\0') {
        Some((_tree_type, tree_entries)) => tree_entries,
        None => "",
    };
    for entry in tree_entries.split('\n') {
        if entry.split(' ').collect::<Vec<&str>>()[0] == "40000" {
            let attributes = entry.split(' ').collect::<Vec<&str>>()[1];
            let relative_file_path = attributes.split('\0').collect::<Vec<&str>>()[0].to_string();
            let file_path = format!("{}/{}", file_path, relative_file_path);
            let file_hash = attributes.split('\0').collect::<Vec<&str>>()[1].to_string();
            get_subtrees_data(file_hash, file_path, tree_hashmap, cliente.clone())?;
        }
        if entry.split(' ').collect::<Vec<&str>>()[0] == "40000" {
            continue;
        }

        let attributes = entry.split(' ').collect::<Vec<&str>>()[1];
        let relative_file_path = attributes.split('\0').collect::<Vec<&str>>()[0].to_string();
        let file_path = format!("{}/{}", file_path, relative_file_path);
        let file_hash = attributes.split('\0').collect::<Vec<&str>>()[1].to_string();

        tree_hashmap.insert(file_path, file_hash);
    }
    Ok(())
}

pub fn get_commit_hashmap(
    commit: String,
    cliente: String,
) -> Result<HashMap<String, String>, GitrError> {
    let mut tree_hashmap = HashMap::new();
    if !commit.is_empty() {
        let repo = file_manager::get_current_repo(cliente.clone())?;
        let tree = file_manager::get_main_tree(commit, cliente.clone())?;
        let tree_data = file_manager::read_object(&tree, repo.clone(), true)?;
        let tree_entries = match tree_data.split_once('\0') {
            Some((_tree_type, tree_entries)) => tree_entries,
            None => "",
        };
        for entry in tree_entries.split('\n') {
            if entry.split(' ').collect::<Vec<&str>>()[0] == "40000" {
                let attributes = entry.split(' ').collect::<Vec<&str>>()[1];
                let _file_path = attributes.split('\0').collect::<Vec<&str>>()[0].to_string();
                let file_path = format!("{}/{}", repo, _file_path);
                let file_hash = attributes.split('\0').collect::<Vec<&str>>()[1].to_string();
                get_subtrees_data(file_hash, file_path, &mut tree_hashmap, cliente.clone())?;
            }

            if entry.split(' ').collect::<Vec<&str>>()[0] == "40000" {
                continue;
            }

            let attributes = entry.split(' ').collect::<Vec<&str>>()[1];
            let _file_path = attributes.split('\0').collect::<Vec<&str>>()[0].to_string();
            let file_path = format!("{}/{}", repo, _file_path);
            let file_hash = attributes.split('\0').collect::<Vec<&str>>()[1].to_string();

            tree_hashmap.insert(file_path, file_hash);
        }
    }

    Ok(tree_hashmap)
}

pub fn get_untracked_notstaged_files(
    cliente: String,
) -> Result<(Vec<String>, Vec<String>, bool), GitrError> {
    let working_dir_hashmap = get_working_dir_hashmap(cliente.clone())?;
    let (index_hashmap, hayindex) = get_index_hashmap(cliente.clone())?;
    let current_commit_hashmap = get_current_commit_hashmap(cliente.clone())?;
    let mut not_staged = Vec::new();
    let mut untracked_files = Vec::new();
    for (path, _) in index_hashmap.clone().into_iter() {
        if !working_dir_hashmap.contains_key(path.as_str()) {
            not_staged.push(path.clone());
        }
    }
    for (path, hash) in working_dir_hashmap.clone().into_iter() {
        if !index_hashmap.contains_key(path.as_str())
            && !current_commit_hashmap.contains_key(path.as_str())
        {
            untracked_files.push(path.clone());
        }
        if current_commit_hashmap.contains_key(path.clone().as_str()) {
            if let Some(commit_hash) = current_commit_hashmap.get(path.as_str()) {
                if &hash != commit_hash && !index_hashmap.contains_key(&path) {
                    not_staged.push(path.clone());
                }
            };
        }
        if index_hashmap.contains_key(path.as_str()) {
            if let Some(index_hash) = index_hashmap.get(path.as_str()) {
                if &hash != index_hash {
                    not_staged.push(path);
                }
            };
        }
    }
    Ok((not_staged, untracked_files, hayindex))
}

pub fn get_current_commit_hashmap(cliente: String) -> Result<HashMap<String, String>, GitrError> {
    let mut tree_hashmap = HashMap::new();
    let mut haycommitshechos = true;
    let current_commit = match file_manager::get_current_commit(cliente.clone()) {
        Ok(commit) => commit,
        Err(_) => {
            haycommitshechos = false;
            String::new()
        }
    };

    if haycommitshechos {
        let repo = file_manager::get_current_repo(cliente.clone())?;
        let tree = file_manager::get_main_tree(current_commit, cliente.clone())?;
        let tree_data = file_manager::read_object(&tree, repo.clone(), true)?;
        let tree_entries = match tree_data.split_once('\0') {
            Some((_tree_type, tree_entries)) => tree_entries,
            None => "",
        };
        for entry in tree_entries.split('\n') {
            let attributes = entry.split(' ').collect::<Vec<&str>>()[1];
            let _file_path = attributes.split('\0').collect::<Vec<&str>>()[0].to_string();
            let file_path = format!("{}/{}", repo, _file_path);
            let file_hash = attributes.split('\0').collect::<Vec<&str>>()[1].to_string();
            tree_hashmap.insert(file_path, file_hash);
        }
    }
    Ok(tree_hashmap)
}
pub fn get_tobe_commited_files(
    not_staged: &[String],
    cliente: String,
) -> Result<(Vec<String>, Vec<String>), GitrError> {
    let (index_hashmap, _) = get_index_hashmap(cliente.clone())?;
    let current_commit_hashmap = get_current_commit_hashmap(cliente.clone())?;
    let mut new_files_to_be_commited = Vec::new();
    let mut modified_files_to_be_commited = Vec::new();
    for (path, hash) in index_hashmap.clone().into_iter() {
        if !current_commit_hashmap.contains_key(path.as_str()) {
            new_files_to_be_commited.push(path);
        } else if let Some(commit_hash) = current_commit_hashmap.get(path.as_str()) {
            if hash != *commit_hash && !not_staged.contains(&path) {
                modified_files_to_be_commited.push(path);
            }
        }
    }
    Ok((new_files_to_be_commited, modified_files_to_be_commited))
}

/***************************
 ***************************
 *    ADD FUNCTIONS
 **************************
 **************************/

pub fn save_and_add_blob_to_index(file_path: String, cliente: String) -> Result<(), GitrError> {
    match path_is_ignored(vec![file_path.clone()], cliente.clone()) {
        Ok(r) => {
            if r {
                return Ok(());
            }
        }
        Err(e) => return Err(e),
    }
    let raw_data = file_manager::read_file(file_path.clone())?;
    let blob = Blob::new(raw_data)?;
    blob.save(cliente.clone())?;
    let hash = blob.get_hash();
    file_manager::add_to_index(&file_path, &hash, cliente.clone())?;
    Ok(())
}

pub fn update_index_before_add(cliente: String) -> Result<(), GitrError> {
    let repo = file_manager::get_current_repo(cliente.clone())?;
    let index_path = &(repo.clone() + "/gitr/index");
    if Path::new(index_path).is_file() {
        let index_data = file_manager::read_index(cliente.clone())?;
        let mut index_vector: Vec<&str> = Vec::new();
        if !index_data.is_empty() {
            index_vector = index_data.split('\n').collect::<Vec<&str>>();
        }
        let mut i: i32 = 0;
        while i != index_vector.len() as i32 {
            let entry = index_vector[i as usize];
            let path_to_check = entry.split(' ').collect::<Vec<&str>>()[3];
            if !Path::new(path_to_check).exists() {
                index_vector.remove(i as usize);
                i -= 1;
            }
            i += 1;
        }
        file_manager::remove_file(index_path.to_string())?;
        for entry in index_vector {
            let path = entry.split(' ').collect::<Vec<&str>>()[3];
            save_and_add_blob_to_index(path.to_string(), cliente.clone())?;
        }
    }
    Ok(())
}

pub fn add_files_command(file_path: String, cliente: String) -> Result<(), GitrError> {
    let repo = get_current_repo(cliente.clone())?;
    if file_path == "." {
        let files = visit_dirs(std::path::Path::new(&repo));
        for file in files {
            if file.contains("gitr") || file.contains("gitrignore") {
                continue;
            }
            save_and_add_blob_to_index(file.clone(), cliente.clone())?;

        }
    } else {
        let full_file_path = repo + "/" + &file_path;
        save_and_add_blob_to_index(full_file_path, cliente)?;
    }
    Ok(())
}

/***************************
 ***************************
 *   RM FUNCTIONS
 **************************
 **************************/

pub fn rm_from_index(file_to_delete: &str, cliente: String) -> Result<bool, GitrError> {
    let mut removed: bool = false;
    let mut index = file_manager::read_index(cliente.clone())?;
    index += "\n";
    let current_repo = file_manager::get_current_repo(cliente)?;
    let file_to_rm_path = format!("{}/{}", current_repo, file_to_delete);
    for line in index.lines() {
        let attributes = line.split(' ').collect::<Vec<&str>>();
        if attributes[3] == file_to_rm_path {
            let complete_line = format!("{}\n", line);
            index = index.replace(&complete_line, "");
            let res = index.trim_end().to_string();
            removed = true;
            let compressed_index = flate2compress(res)?;
            let _ = file_manager::write_compressed_data(
                &(current_repo + "/gitr/index"),
                compressed_index.as_slice(),
            );
            break;
        }
    }
    Ok(removed)
}

/***************************
 ***************************
 *    TAG FUNCTIONS
 **************************
 **************************/

pub fn create_lightweight_tag(tag_name: String, cliente: String) -> Result<(), GitrError> {
    let current_commit = match get_current_commit(cliente.clone()) {
        Ok(commit) => commit,
        Err(_) => {
            println!("fatal: Failed to resolve 'HEAD' as a valid ref.");
            return Ok(());
        }
    };
    let tag_path = get_current_repo(cliente.clone())? + "/gitr/refs/tags/" + &tag_name;
    if Path::new(&tag_path).exists() {
        return Err(GitrError::TagAlreadyExistsError(tag_name.clone()));
    }
    file_manager::write_file(tag_path, current_commit)?;
    Ok(())
}

pub fn create_annotated_tag(
    tag_name: String,
    tag_message: String,
    cliente: String,
) -> Result<(), GitrError> {
    let current_commit = match get_current_commit(cliente.clone()) {
        Ok(commit) => commit,
        Err(_) => {
            println!("fatal: Failed to resolve 'HEAD' as a valid ref.");
            return Ok(());
        }
    };
    let tag_path = get_current_repo(cliente.clone())? + "/gitr/refs/tags/" + &tag_name;
    if Path::new(&tag_path).exists() {
        return Err(GitrError::TagAlreadyExistsError(tag_name.clone()));
    }
    let tag = Tag::new(tag_name, tag_message, current_commit, cliente.clone())?;
    tag.save(cliente.clone())?;
    file_manager::write_file(tag_path, tag.get_hash())?;
    Ok(())
}

pub fn get_tags_str(cliente: String) -> Result<String, GitrError> {
    let tags = file_manager::get_tags(cliente)?;
    let mut tag_str = String::new();
    for t in tags {
        tag_str.push_str(&(t + "\n"))
    }
    Ok(tag_str.strip_suffix('\n').unwrap_or("").to_string())
}
/***************************
 ***************************
 *    CLONE FUNCTIONS
 **************************
 **************************/

pub fn write_reference_from_cloning(
    references: Vec<(String, String)>,
    _ref_disc: String,
    cliente: String,
) -> Result<(), GitrError> {
    let repo = file_manager::get_current_repo(cliente.clone())?;
    for reference in &references[1..] {
        let path_str = repo.clone() + "/gitr/" + &reference.1.clone();
        if references[0].0 == reference.0 {
            file_manager::update_head(&reference.1.clone(), cliente.clone())?;
        }
        let into_hash = reference.0.clone();
        file_manager::write_file(path_str, into_hash)?;
    }
    Ok(())
}

pub fn clone_connect_to_server(address: String) -> Result<TcpStream, GitrError> {
    let socket = match TcpStream::connect(address) {
        Ok(socket) => socket,
        Err(_) => {
            return Err(GitrError::InvalidArgumentError(
                "address".to_string(),
                "localhost:9418".to_string(),
            ))
        }
    };
    Ok(socket)
}

pub fn clone_send_git_upload_pack(socket: &mut TcpStream) -> Result<usize, GitrError> {
    match socket.write("0031git-upload-pack /mi-repo\0host=localhost:9418\0".as_bytes()) {
        //51 to hexa =
        Ok(bytes) => Ok(bytes),
        Err(e) => Err(GitrError::SocketError(
            "clone_send_git_upload_pack()".to_string(),
            e.to_string(),
        )),
    }
}

pub fn clone_read_reference_discovery(socket: &mut TcpStream) -> Result<String, GitrError> {
    let mut buffer = [0; 1024];
    let mut response = String::new();
    loop {
        let bytes_read = match socket.read(&mut buffer) {
            Ok(bytes) => bytes,
            Err(e) => {
                return Err(GitrError::SocketError(
                    "clone_read_reference_discovery()".to_string(),
                    e.to_string(),
                ))
            }
        };
        let received_message = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
        if bytes_read == 0 || received_message == "0000" {
            break;
        }
        response.push_str(&received_message);
    }
    Ok(response)
}

pub fn write_socket(socket: &mut TcpStream, message: &[u8]) -> Result<(), GitrError> {
    match socket.write(message) {
        Ok(_) => Ok(()),
        Err(e) => Err(GitrError::SocketError(
            "write_socket()".to_string(),
            e.to_string(),
        )),
    }
}

pub fn read_socket(socket: &mut TcpStream, buffer: &mut [u8]) -> Result<(), GitrError> {
    let bytes_read = match socket.read(buffer) {
        Ok(bytes) => bytes,
        Err(e) => {
            return Err(GitrError::SocketError(
                "read_socket()".to_string(),
                e.to_string(),
            ))
        }
    };
    let _received_data = String::from_utf8_lossy(&buffer[..bytes_read]);
    Ok(())
}

/***************************
 ***************************
 *    PULL FUNCTIONS
 **************************
 **************************/

pub fn handshake(orden: String, cliente: String) -> Result<TcpStream, GitrError> {

    let remote = file_manager::get_remote(cliente.clone())?;
    let url_n_name = remote.split('/').collect::<Vec<&str>>();
    if url_n_name.len() != 2 {
        return Err(GitrError::InvalidArgumentError(
            "<no se recibio un remote>".to_string(),
            "url/name".to_string(),
        ));
    }
    let msj = format!("{} /{}\0host={}\0", orden, url_n_name[1], url_n_name[0]);
    let msj = format!("{:04x}{}", msj.len() + 4, msj);
    let mut stream = match TcpStream::connect(url_n_name[0]) {
        Ok(s) => s,
        Err(e) => {
            println!("Error: {}", e);
            return Err(GitrError::ConnectionError);
        }
    };
    match stream.write(msj.as_bytes()) {
        Ok(_) => (),
        Err(e) => {
            println!("Error: {}", e);
            return Err(GitrError::ConnectionError);
        }
    };
    Ok(stream)
}

pub fn protocol_reference_discovery(
    stream: &mut TcpStream,
) -> Result<Vec<(String, String)>, GitrError> {
    let mut buffer = Vec::new();
    while !buffer.ends_with("0000".as_bytes()) {
        if buffer.starts_with("Error".as_bytes()) {
            println!("{}", String::from_utf8_lossy(&buffer));
            return Err(GitrError::ConnectionError);
        }
        let aux = match read_long_stream(stream) {
            Ok(buf) => buf,
            Err(e) => {
                println!("{}",e);
                return Err(GitrError::ConnectionError);
            }
        };
        buffer.extend(aux);
    }
    let ref_disc = String::from_utf8_lossy(&buffer).to_string();
    let hash_n_references = ref_discovery::discover_references(ref_disc)?;
    Ok(hash_n_references)
}

pub fn protocol_wants_n_haves(
    hash_n_references: Vec<(String, String)>,
    stream: &mut TcpStream,
    cliente: String,
) -> Result<bool, GitrError> {
    let mut refs_ids = file_manager::get_refs_ids("heads", cliente.clone())?;
    refs_ids.append(file_manager::get_refs_ids("tags", cliente.clone())?.as_mut());
    let want_message =
        ref_discovery::assemble_want_message(&hash_n_references, refs_ids, cliente.clone())?;
    match stream.write(want_message.as_bytes()) {
        Ok(_) => (),
        Err(e) => {
            println!("Error: {}", e);
            return Err(GitrError::ConnectionError);
        }
    };
    if want_message == "0000" {
        return Ok(false);
    }
    let _ = stream.write(&0_usize.to_be_bytes());

    let mut buffer = [0; 1024];
    match stream.read(&mut buffer) {
        Ok(_n) => {
            if String::from_utf8_lossy(&buffer).contains("Error") {
                println!("Error: {}", String::from_utf8_lossy(&buffer));
                return Ok(false);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
            return Err(GitrError::ConnectionError);
        }
    }
    Ok(true)
}

pub fn pull_packfile(stream: &mut TcpStream, cliente: String) -> Result<(), GitrError> {
    let mut buf = match ref_discovery::read_long_stream(stream) {
        Ok(buf) => buf,
        Err(e) => {
            println!("Error: {}", e);
            return Err(GitrError::ConnectionError);
        }
    };
    if buf.is_empty() {
        println!("Error: packfile vacío");
        return Ok(());
    }
    let pack_file_struct = PackFile::new_from_server_packfile(&mut buf)?;
    for object in pack_file_struct.objects.iter() {
        match object {
            GitObject::Blob(blob) => blob.save(cliente.clone())?,
            GitObject::Commit(commit) => commit.save(cliente.clone())?,
            GitObject::Tree(tree) => tree.save(cliente.clone())?,
            GitObject::Tag(tag) => tag.save(cliente.clone())?,
        }
    }
    Ok(())
}

/***************************
 ***************************
 *    PUSH FUNCTIONS
 **************************
 **************************/

pub fn reference_update_request(
    stream: &mut TcpStream,
    hash_n_references: Vec<(String, String)>,
    cliente: String,
) -> Result<(bool, Vec<String>), GitrError> {
    let ids_propios = (
        file_manager::get_refs_ids("heads", cliente.clone())?,
        file_manager::get_refs_ids("tags", cliente.clone())?,
    ); // esta sacando de gitr/refs/heads y tags
    let refs_propios = (
        get_branches(cliente.clone())?,
        get_tags(cliente.clone())?,
    ); // tambien de gitr/refs/heads y tags
    let (ref_upd, pkt_needed, pkt_ids) = match ref_discovery::reference_update_request(
        hash_n_references.clone(),
        ids_propios,
        refs_propios,
    ) {
        Ok((ref_upd, pkt_needed, pkt_ids)) => (ref_upd, pkt_needed, pkt_ids),
        Err(e) => {
            match stream.write("0000".as_bytes()) {
                Ok(_) => (),
                Err(_) => {return Err(GitrError::ConnectionError)}
            }
            return Err(e);
        }
    };
    
    if let Err(e) = stream.write(ref_upd.as_bytes()) {
        println!("Error: {}", e);
        return Err(GitrError::ConnectionError);
    };
    if ref_upd == "0000" {
        println!("client up to date");
        return Ok((false, Vec::new()));
    }
    Ok((pkt_needed, pkt_ids))
}

pub fn push_packfile(
    stream: &mut TcpStream,
    pkt_ids: Vec<String>,
    hash_n_references: Vec<(String, String)>,
    cliente: String,
) -> Result<(), GitrError> {
    let repo = file_manager::get_current_repo(cliente.clone())? + "/gitr";
    let all_pkt_commits = Commit::get_parents(
        pkt_ids.clone(),
        hash_n_references.iter().map(|t| t.0.clone()).collect(),
        repo.clone(),
    )?;
    let ids = Commit::get_objects_from_commits(all_pkt_commits, vec![], repo.clone())?;
    let mut contents: Vec<Vec<u8>> = Vec::new();
    for id in ids {
        contents.push(file_manager::get_object_bytes(id, repo.clone())?)
    }
    let cont: Vec<(String, String, Vec<u8>)> =
        crate::git_transport::pack_file::prepare_contents(contents.clone());
    let pk = crate::git_transport::pack_file::create_packfile(cont.clone())?;
    if let Err(e) = stream.write(&pk) {
        println!("Error: {}", e);
        return Err(GitrError::ConnectionError);
    };
    Ok(())
}
/*******************
 * REBASE FUNCTIONS
 * *****************/

fn check_conflicts_and_get_tree(
    origin_commit: String,
    branch_commit: String,
    base_commit: String,
    cliente: String,
) -> Result<String, GitrError> {
    let (hubo_conflict, _) =
        three_way_merge(base_commit, branch_commit, origin_commit, cliente.clone())?;
    if hubo_conflict {
        loop {
            println!("conflicts detected, please resolve them and then run '--continue'");
            print!("$ ");
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => (),
                Err(_e) => return Err(GitrError::InputError),
            }
            if input.trim() == "--continue" {
                break;
            }
        }
    }
    commands_fn::add(vec![".".to_string()], cliente.clone())?;
    let (tree_map, tree_order) = get_hashmap_for_checkout(cliente.clone())?;
    let final_tree = create_trees(tree_map, tree_order[0].clone(), cliente.clone())?;
    Ok(final_tree.get_hash())
}

pub fn create_rebase_commits(
    to_rebase_commits: Vec<String>,
    origin_name: String,
    cliente: String,
    commit_base: String,
) -> Result<(), GitrError> {
    let mut last_commit: String = get_commit(origin_name, cliente.clone())?;
    let head = get_head(cliente.clone())?;
    let path = get_current_repo(cliente.clone())? + "/gitr/" + &head;
    file_manager::write_file(path, last_commit.clone())?;
    for commit_old in to_rebase_commits.iter().rev() {
        let main_tree = check_conflicts_and_get_tree(
            last_commit.clone(),
            commit_old.to_string(),
            commit_base.clone(),
            cliente.clone(),
        )?;
        let message = file_manager::get_commit_message(commit_old.clone(), cliente.clone())?;
        let commit = Commit::new(
            main_tree.clone(),
            vec![last_commit.clone()],
            cliente.clone(),
            cliente.clone(),
            message.clone(),
            cliente.clone(),
        )?;
        commit.save(cliente.clone())?;
        let dir = get_current_repo(cliente.clone())? + "/gitr/" + &get_head(cliente.clone())?;
        file_manager::write_file(dir, commit.get_hash())?;
        last_commit = commit.get_hash();
    }
    Ok(())
}

/*******************
 * CHECK-IGNORE FUNCTIONS
 * *****************/
fn path_is_ignored(paths: Vec<String>, client: String) -> Result<bool, GitrError> {
    let ignored_paths = match check_ignore_(paths, client) {
        Ok(paths) => paths,
        Err(e) => return Err(e),
    };
    if ignored_paths.is_empty() {
        return Ok(false);
    }
    Ok(true)
}

pub fn check_ignore_(paths: Vec<String>, client: String) -> Result<Vec<String>, GitrError> {
    let path = armar_path("gitrignore".to_string(), client.clone())?;
    let gitignore = match file_manager::read_file(path){
        Ok(data) => data,
        Err(_) => return Ok(vec![]),
    };
    let lineas_ignore: Vec<&str> = gitignore.split('\n').collect();
    let mut lineas_full: Vec<String> = vec![];
    let repo = file_manager::get_current_repo(client.clone())?;
    for linea in lineas_ignore {
        lineas_full.push(repo.to_owned() + linea);
    }
    let mut ignored_paths = vec![];
    for path in paths {
        if lineas_full.contains(&path) {
            println!("path: {} is ignored", path);
            ignored_paths.push(path);
        }
    }
    Ok(ignored_paths)
}

pub fn armar_path(path: String, cliente: String) -> Result<String, GitrError> {
    let full_path = vec![
        file_manager::get_current_repo(cliente)?,
        "/".to_string(),
        path,
    ];
    Ok(full_path.concat())
}

/*******************
 *   LS-TREE FUNCTIONS
 * *****************/

pub fn _ls_tree(
    flags: Vec<String>,
    father_dir: String,
    cliente: String,
) -> Result<String, GitrError> {
    let tree_hash = flags[flags.len() - 1].clone();
    let data = _cat_file(vec!["-p".to_string(), tree_hash.clone()], cliente.clone())?;

    if flags.len() == 1 {
        print!("{}", data);
        return Ok(data);
    }

    let entries = data.split('\n').collect::<Vec<&str>>();
    let mut result = String::new();
    for entry in entries {
        if entry.is_empty() {
            continue;
        }
        let entry = entry.split(' ').collect::<Vec<&str>>();

        let mut res_entry = Vec::new();

        if flags[0].contains('r') && entry[1] == "tree" {
            let new_father = if father_dir.is_empty() {
                entry[3].to_string()
            } else {
                father_dir.clone() + "/" + entry[3]
            };
            _ls_tree(
                vec![flags[0].clone(), entry[2].to_string().clone()],
                new_father.clone(),
                cliente.clone(),
            )?;
            if !flags[0].contains('t') {
                continue;
            }
        }

        if flags[0].contains('d') && entry[1] != "tree" {
            continue;
        }

        res_entry.push(entry[0].to_string());
        res_entry.push(entry[1].to_string());
        res_entry.push(entry[2].to_string());

        if flags[0].contains('l') {
            let size = _cat_file(
                vec!["-s".to_string(), entry[2].to_string().clone()],
                cliente.clone(),
            )?;
            res_entry.push(size);
        }

        let full_path = if father_dir.is_empty() {
            entry[3].to_string()
        } else {
            father_dir.clone() + "/" + entry[3]
        };

        res_entry.push(full_path); // nombre

        if flags[0].contains('z') {
            // separar con \0
            res_entry.push("\0".to_string());
        } else {
            res_entry.push("\n".to_string());
        }
        result.push_str(&res_entry.join(" "));
    }

    print!("{}", result);
    Ok(result)
}

/*****************
 * PULL REQUESTS *
 *****************/

pub fn _create_pr(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    let remote = file_manager::get_remote(cliente.clone())?;
    let sv_url = remote.split('/').collect::<Vec<&str>>()[0];
    let sv_name = remote.split('/').collect::<Vec<&str>>()[1];
    let title = flags[0].clone();
    let description = flags[1].clone();
    let head = flags[2].clone();
    let base = flags[3].clone();
    let body = format!("{{\"id\":1,\"title\":\"{}\",\"description\":\"{}\",\"head\":\"{}\",\"base\":\"{}\",\"status\":\"open\"}}", title, description, head, base);
    let _server_addr = format!("/repos/{}/pulls HTTP/1.1\n", sv_name);
    let path = format!("{}/repos/{}/pulls", sv_url, sv_name);
    let child = Command::new("curl")
            .arg("-isS")
            .arg("-X")
            .arg("POST")
            .arg("-d")
            .arg(body)
            .arg(path)
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to execute curl");
        
    let _output = child.wait_with_output().expect("failed to wait on child");

    Ok(())
}




// Esta suite solo corre bajo el Git Daemon que tiene Bruno, está hardcodeado el puerto y la dirección, además del repo remoto.
//#[cfg(test)]
// mod clone_tests {
//  use super::*;

// #[test]
// fn test00_clone_connects_to_daemon_correctly(){
//     assert!(clone_connect_to_server("localhost:9418".to_string()).is_ok());
// }

// #[test]
// fn test01_clone_send_git_upload_pack_to_daemon_correctly(){
//     let mut socket = clone_connect_to_server("localhost:9418".to_string()).unwrap();
//     assert_eq!(clone_send_git_upload_pack(&mut socket).unwrap(),49); //0x31 = 49
// }

// #[test]
// fn test02_clone_receive_daemon_reference_discovery_correctly(){ //test viejo ya no corre
//     let mut socket = clone_connect_to_server("localhost:9418".to_string()).unwrap();
//     clone_send_git_upload_pack(&mut socket).unwrap();
//     assert_eq!(clone_read_reference_discovery(&mut socket).unwrap(),"0103cf6335a864bda2ee027ea7083a72d10e32921b15 HEAD\0multi_ack thin-pack side-band side-band-64k ofs-delta shallow deepen-since deepen-not deepen-relative no-progress include-tag multi_ack_detailed symref=HEAD:refs/heads/main object-format=sha1 agent=git/2.34.1\n003dcf6335a864bda2ee027ea7083a72d10e32921b15 refs/heads/main\n");
// }

// #[test]
// fn test03_clone_gets_reference_vector_correctly(){ //test viejo ya no corre
//     let mut socket = clone_connect_to_server("localhost:9418".to_string()).unwrap();
//     clone_send_git_upload_pack(&mut socket).unwrap();
//     let ref_disc = clone_read_reference_discovery(&mut socket).unwrap();
//     assert_eq!(ref_discovery::discover_references(ref_disc).unwrap(),
//     [("cf6335a864bda2ee027ea7083a72d10e32921b15".to_string(), "HEAD".to_string()),
//     ("cf6335a864bda2ee027ea7083a72d10e32921b15".to_string(), "refs/heads/main".to_string())]);
// }

// #[test]
// fn test04_clone_sends_wants_correctly(){
//     let mut socket = clone_connect_to_server("localhost:9418".to_string()).unwrap();
//     clone_send_git_upload_pack(&mut socket).unwrap();
//     let ref_disc = clone_read_reference_discovery(&mut socket).unwrap();
//     let references = ref_discovery::discover_references(ref_disc).unwrap();
//     socket.write(assemble_want_message(&references,vec![]).unwrap().as_bytes()).unwrap();
// }

// fn test04_clone_sends_wants_correctly(){
//     let mut socket = clone_connect_to_server("localhost:9418".to_string()).unwrap();
//     clone_send_git_upload_pack(&mut socket).unwrap();
//     let ref_disc = clone_read_reference_discovery(&mut socket).unwrap();
//     let references = ref_discovery::discover_references(ref_disc).unwrap();
//     socket.write(assemble_want_message(&references,vec![],"Test".to_string()).unwrap().as_bytes()).unwrap();
// }
//}

#[cfg(test)]
mod comparar_diffs_tests {

    use super::*;

    #[test]
    fn comparar_diffs_test_01_diffs_sin_conflicts_desde_origin() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_origin = "hola\nque\nestas\nbien\ny\nvos\n".to_string();
        let str_new = "hola\nque\nestas\n".to_string();
        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 2);
        let lineas_esperadas = vec![
            (1, false, "como".to_string()),
            (1, true, "que".to_string()),
            (3, true, "bien\ny\nvos".to_string()),
        ];
        assert_eq!(diff_final.unwrap().0.lineas, lineas_esperadas);
    }
    #[test]
    fn comparar_diffs_test_02_diffs_sin_conflicts_desde_new() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_origin = "hola\nque\nestas\n".to_string();
        let str_new = "hola\nque\nestas\nbien\ny\nvos\n".to_string();
        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 2);
        let lineas_esperadas = vec![
            (1, false, "como".to_string()),
            (1, true, "que".to_string()),
            (3, true, "bien\ny\nvos".to_string()),
        ];
        assert_eq!(diff_final.unwrap().0.lineas, lineas_esperadas);
    }

    #[test]
    fn comparar_diffs_test_03_diffs_con_conflict() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_origin = "hola\nque\ntal\n".to_string();
        let str_new = "hola\nque\ntal\nbien\ny\nvos\n".to_string();
        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 2);
        let lineas_esperadas = vec![
            (1, false, "como".to_string()),
            (
                1,
                true,
                "<<<<<<< HEAD\nque\ntal\n=======\nque\ntal\nbien\ny\nvos\n>>>>>>> BRANCH"
                    .to_string(),
            ),
            (2, false, "estas".to_string()),
        ];
        assert_eq!(diff_final.unwrap().0.lineas, lineas_esperadas);
    }

    #[test]
    fn comparar_diffs_test_04_diffs_con_1_conflict_en_primera_linea() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_origin = "buenas\ncomo\nestas\n".to_string();
        let str_new = "nihao\ncomo\nestas\n".to_string();
        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 2);
        let lineas_esperadas = vec![
            (0, false, "hola".to_string()),
            (
                0,
                true,
                "<<<<<<< HEAD\nbuenas\n=======\nnihao\n>>>>>>> BRANCH".to_string(),
            ),
        ];
        assert_eq!(diff_final.unwrap().0.lineas, lineas_esperadas);
    }

    #[test]
    fn comparar_diffs_test_05_diffs_con_1_conflict_en_la_ultima_linea() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_origin = "hola\ncomo\nandas\n".to_string();
        let str_new = "hola\ncomo\ntas\n".to_string();
        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 2);
        let lineas_esperadas = vec![
            (2, false, "estas".to_string()),
            (
                2,
                true,
                "<<<<<<< HEAD\nandas\n=======\ntas\n>>>>>>> BRANCH".to_string(),
            ),
        ];
        assert_eq!(diff_final.unwrap().0.lineas, lineas_esperadas);
    }

    #[test]
    fn comparar_diffs_test_06_diffs_con_1_conflict_en_3_lineas() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_origin = "hola\nromo\nestas\n".to_string();
        let str_new = "hola\nfomo\nestas\n".to_string();
        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 2);
        let lineas_esperadas = vec![
            (1, false, "como".to_string()),
            (
                1,
                true,
                "<<<<<<< HEAD\nromo\n=======\nfomo\n>>>>>>> BRANCH".to_string(),
            ),
        ];
        assert_eq!(diff_final.unwrap().0.lineas, lineas_esperadas);
    }

    #[test]
    fn comparar_diffs_test_07_conflicts_en_archivo_de_una_sola_linea() {
        let str_base = "hola\n".to_string();
        let str_origin = "origin\n".to_string();
        let str_new = "new\n".to_string();
        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 0);
        let lineas_esperadas = vec![
            (0, false, "hola".to_string()),
            (
                0,
                true,
                "<<<<<<< HEAD\norigin\n=======\nnew\n>>>>>>> BRANCH".to_string(),
            ),
        ];
        assert_eq!(diff_final.unwrap().0.lineas, lineas_esperadas);
    }

    #[test]
    fn comparar_diffs_test_08_conflicts_en_todas_las_lineas_de_archivo_de_dos_lineas() {
        let str_base = "hola\ncomo\n".to_string();
        let str_origin = "origin1\norigin2\n".to_string();
        let str_new = "new1\nnew2\n".to_string();
        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 1);
        let lineas_esperadas = vec![
            (0, false, "hola".to_string()),
            (
                0,
                true,
                "<<<<<<< HEAD\norigin1\norigin2\n=======\nnew1\nnew2\n>>>>>>> BRANCH".to_string(),
            ),
            (1, false, "como".to_string()),
        ];
        assert_eq!(diff_final.unwrap().0.lineas, lineas_esperadas);
    }

    #[test]
    fn comparar_diffs_test_09_conflicts_en_todas_las_lineas_de_archivo_de_cinco_lineas() {
        let str_base = "hola\ncomo\nestas\npepe\ngrillo".to_string();
        let str_origin = "origin1\norigin2\norigin3\norigin4\norigin5".to_string();
        let str_new = "new1\nnew2\nnew3\nnew4\nnew5\n".to_string();
        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 4);
        let lineas_esperadas = vec![
            (0,false,"hola".to_string()),
            (0,true,"<<<<<<< HEAD\norigin1\norigin2\norigin3\norigin4\norigin5\n=======\nnew1\nnew2\nnew3\nnew4\nnew5\n>>>>>>> BRANCH".to_string()),
            (1,false,"como".to_string()),
            (2,false,"estas".to_string()),
            (3,false,"pepe".to_string()),
            (4,false,"grillo".to_string()),
        ];
        assert_eq!(diff_final.unwrap().0.lineas, lineas_esperadas);
    }

    #[test]
    fn comparar_diffs_test_10_conflicts_con_diffs_de_distinto_tamanio_mas_lineas_en_branch() {
        let str_base = "hola\n".to_string();
        let str_origin = "hola\norigin1\n".to_string();
        let str_new = "hola\ncomo\nnew3\n".to_string();
        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let (diff_final, _) = comparar_diffs(diff_base_origin, diff_base_branch, 0).unwrap();
        let lineas_esperadas = vec![(
            1,
            true,
            "<<<<<<< HEAD\norigin1\n=======\ncomo\nnew3\n>>>>>>> BRANCH".to_string(),
        )];

        assert_eq!(diff_final.lineas, lineas_esperadas);
    }
    #[test]
    fn comparar_diffs_test_11_conflicts_con_diffs_de_distinto_tamanio_mas_lineas_en_origin() {
        let str_base = "hola\n".to_string();
        let str_origin = "hola\ncomo\norigin\n".to_string();
        let str_new = "hola\nnew\n".to_string();
        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 0).unwrap();
        let lineas_esperadas = vec![(
            1,
            true,
            "<<<<<<< HEAD\ncomo\norigin\n=======\nnew\n>>>>>>> BRANCH".to_string(),
        )];

        assert_eq!(diff_final.0.lineas, lineas_esperadas);
    }

    #[test]
    fn comparar_diffs_test_12_conflicts_con_diffs_varios_largos_en_distintos_lugares() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_origin =
            "hola\ncomo\nori1\nori2\nori3\nestas\nori4\niguales\nori5\niguales para cerrar"
                .to_string();
        let str_new =
            "hola\ncomo\nnew1\nestas\nnew2\nnew3\niguales\nnew4\niguales para cerrar".to_string();

        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 2).unwrap();
        let lineas_esperadas = vec![
            (2,true,"<<<<<<< HEAD\nori1\nori2\nori3\n=======\nnew1\n>>>>>>> BRANCH".to_string()),
            (4,true,"<<<<<<< HEAD\nori4\niguales\nori5\niguales para cerrar\n=======\nnew2\nnew3\niguales\nnew4\niguales para cerrar\n>>>>>>> BRANCH".to_string()),
        ];
        assert_eq!(diff_final.0.lineas, lineas_esperadas);
    }

    #[test]
    fn comparar_diffs_test_13_comparar_diffs_distinto_largo_simple() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_origin = "hola\ncomo\nori1\nori2\nori3\nestas\n".to_string();
        let str_new = "hola\ncomo\nnew1\nestas\n".to_string();

        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 2).unwrap();
        let diff_esperado = vec![(
            2,
            true,
            "<<<<<<< HEAD\nori1\nori2\nori3\n=======\nnew1\n>>>>>>> BRANCH".to_string(),
        )];

        assert_eq!(diff_final.0.lineas, diff_esperado);
    }
}

#[cfg(test)]
mod aplicar_diffs_tests {

    use super::*;

    #[test]
    fn aplicar_diff_test_01_cambio_al_inicio() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_new = "new1\ncomo\nestas\n".to_string();

        let diff_base_branch = Diff::new(str_base.clone(), str_new);
        let _archivo_reconstruido = _aplicar_diffs(str_base, diff_base_branch).unwrap();
        let archivo_esperado = vec!["new1\n", "como\n", "estas\n"];

        assert_eq!(_archivo_reconstruido, archivo_esperado);
    }

    #[test]
    fn aplicar_diff_test_02_cambio_en_el_medio() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_new = "hola\nnew1\nestas\n".to_string();

        let diff_base_branch = Diff::new(str_base.clone(), str_new);
        let _archivo_reconstruido = _aplicar_diffs(str_base, diff_base_branch).unwrap();
        let archivo_esperado = vec!["hola\n", "new1\n", "estas\n"];

        assert_eq!(_archivo_reconstruido, archivo_esperado);
    }

    #[test]
    fn aplicar_diff_test_03_cambio_al_final() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_new = "hola\ncomo\nnew1\n".to_string();

        let diff_base_branch = Diff::new(str_base.clone(), str_new);
        let _archivo_reconstruido = _aplicar_diffs(str_base, diff_base_branch).unwrap();
        let archivo_esperado = vec!["hola\n", "como\n", "new1\n"];

        assert_eq!(_archivo_reconstruido, archivo_esperado);
    }

    #[test]
    fn aplicar_diff_test_04_conflict_al_inicio() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_origin = "origin1\ncomo\nestas\n".to_string();
        let str_new = "new1\ncomo\nestas\n".to_string();

        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 2).unwrap();
        let _archivo_reconstruido = _aplicar_diffs(str_base, diff_final.0).unwrap();
        let archivo_esperado = vec![
            "<<<<<<< HEAD\norigin1\n=======\nnew1\n>>>>>>> BRANCH\n",
            "como\n",
            "estas\n",
        ];

        assert_eq!(_archivo_reconstruido, archivo_esperado);
    }

    #[test]
    fn aplicar_diff_test_05_conflict_al_medio() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_origin = "hola\norigin1\nestas\n".to_string();
        let str_new = "hola\nnew1\nestas\n".to_string();

        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 2).unwrap();
        let _archivo_reconstruido = _aplicar_diffs(str_base, diff_final.0).unwrap();
        let archivo_esperado = vec![
            "hola\n",
            "<<<<<<< HEAD\norigin1\n=======\nnew1\n>>>>>>> BRANCH\n",
            "estas\n",
        ];

        assert_eq!(_archivo_reconstruido, archivo_esperado);
    }

    #[test]
    fn aplicar_diff_test_06_conflict_al_final() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_origin = "hola\ncomo\norigin1\n".to_string();
        let str_new = "hola\ncomo\nnew1\n".to_string();

        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 2).unwrap();
        let _archivo_reconstruido = _aplicar_diffs(str_base, diff_final.0).unwrap();
        let archivo_esperado = vec![
            "hola\n",
            "como\n",
            "<<<<<<< HEAD\norigin1\n=======\nnew1\n>>>>>>> BRANCH\n",
        ];

        assert_eq!(_archivo_reconstruido, archivo_esperado);
    }

    #[test]
    fn aplicar_diff_test_07_conflict_multilinea_al_inicio() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_new = "new1\nnew2\ncomo\nestas\n".to_string();
        let str_origin = "origin1\norigin2\ncomo\nestas\n".to_string();

        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 2).unwrap();
        let _archivo_reconstruido = _aplicar_diffs(str_base, diff_final.0).unwrap();
        let archivo_esperado = vec![
            "<<<<<<< HEAD\norigin1\norigin2\n=======\nnew1\nnew2\n>>>>>>> BRANCH\n",
            "como\n",
            "estas\n",
        ];

        assert_eq!(_archivo_reconstruido, archivo_esperado);
    }

    #[test]
    fn aplicar_diff_test_08_conflict_multiniea_al_medio() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_new = "hola\nnew1\nnew2\nestas\n".to_string();
        let str_origin = "hola\norigin1\norigin2\nestas\n".to_string();

        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 2).unwrap();
        let _archivo_reconstruido = _aplicar_diffs(str_base, diff_final.0).unwrap();
        let archivo_esperado = vec![
            "hola\n",
            "<<<<<<< HEAD\norigin1\norigin2\n=======\nnew1\nnew2\n>>>>>>> BRANCH\n",
            "estas\n",
        ];

        assert_eq!(_archivo_reconstruido, archivo_esperado);
    }

    #[test]
    fn aplicar_diff_test_09_conflict_multilinea_al_final() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_new = "hola\ncomo\nnew1\nnew2\n".to_string();
        let str_origin = "hola\ncomo\norigin1\norigin2\n".to_string();

        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 2).unwrap();
        let _archivo_reconstruido = _aplicar_diffs(str_base, diff_final.0).unwrap();
        let archivo_esperado = vec![
            "hola\n",
            "como\n",
            "<<<<<<< HEAD\norigin1\norigin2\n=======\nnew1\nnew2\n>>>>>>> BRANCH\n",
        ];

        assert_eq!(_archivo_reconstruido, archivo_esperado);
    }

    #[test]
    fn aplicar_diff_test_10_conflicts_con_diffs_varios_largos_en_distintos_lugares() {
        //Lo esperado es sacado de replicar el conflict en git real
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_origin =
            "hola\ncomo\nori1\nori2\nori3\nestas\nori4\niguales\nori5\niguales para cerrar"
                .to_string();
        let str_new =
            "hola\ncomo\nnew1\nestas\nnew2\nnew3\niguales\nnew4\niguales para cerrar".to_string();

        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 2).unwrap();
        let _archivo_reconstruido = _aplicar_diffs(str_base, diff_final.0).unwrap();
        let archivo_esperado = vec!["hola\n",
        "como\n",
        "<<<<<<< HEAD\nori1\nori2\nori3\n=======\nnew1\n>>>>>>> BRANCH\n",
        "estas\n",
        "<<<<<<< HEAD\nori4\niguales\nori5\niguales para cerrar\n=======\nnew2\nnew3\niguales\nnew4\niguales para cerrar\n>>>>>>> BRANCH\n",
        ];

        for i in 0..archivo_esperado.len() {
            assert_eq!(_archivo_reconstruido[i], archivo_esperado[i]);
        }
    }

    #[test]
    fn aplicar_diff_test_11_conflicts_con_diffs_varios_largos_en_distintos_lugares() {
        //Lo esperado es sacado de replicar el conflict en git real
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_origin = "hola\ncomo\nori1\nori2\nori3\nestas\n".to_string();
        let str_new = "hola\ncomo\nnew1\nestas\n".to_string();

        let diff_base_origin = Diff::new(str_base.clone(), str_origin);
        let diff_base_branch = Diff::new(str_base.clone(), str_new);

        let diff_final = comparar_diffs(diff_base_origin, diff_base_branch, 2).unwrap();
        let _archivo_reconstruido = _aplicar_diffs(str_base, diff_final.0).unwrap();
        let archivo_esperado = vec![
            "hola\n",
            "como\n",
            "<<<<<<< HEAD\nori1\nori2\nori3\n=======\nnew1\n>>>>>>> BRANCH\n",
            "estas\n",
        ];

        assert_eq!(_archivo_reconstruido, archivo_esperado);
    }

    #[test]
    fn aplicar_diff_test_12_diff_agregando_una_linea_al_medio() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_new = "hola\ncomo\nnew1\nestas\n".to_string();

        let diff_base_branch = Diff::new(str_base.clone(), str_new);
        let _archivo_reconstruido = _aplicar_diffs(str_base, diff_base_branch).unwrap();
        let archivo_esperado = vec!["hola\n", "como\n", "new1\n", "estas\n"];

        assert_eq!(_archivo_reconstruido, archivo_esperado);
    }
}

#[cfg(test)]
mod juntar_consecutivos_tests {

    use super::*;

    #[test]
    fn juntar_consec_test_01() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_new = "hola\ncomo\nnew1\nnew2\nestas\n".to_string();

        let diff_base_branch = Diff::new(str_base.clone(), str_new);
        let diff_sin_consec = juntar_consecutivos(diff_base_branch);

        let diff_esperado = vec![(2, true, "new1\nnew2".to_string())];

        assert_eq!(diff_sin_consec.lineas, diff_esperado);
    }
    #[test]
    fn juntar_consec_test_02_mezclados() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_new = "hola\ncomo\nnew1\nnew2\nestas\nnew3\nnew4\n".to_string();

        let diff_base_branch = Diff::new(str_base.clone(), str_new);
        let diff_sin_consec = juntar_consecutivos(diff_base_branch);

        let diff_esperado = vec![
            (2, true, "new1\nnew2".to_string()),
            (5, true, "new3\nnew4".to_string()),
        ];

        assert_eq!(diff_sin_consec.lineas, diff_esperado);
    }

    #[test]
    fn juntar_consec_test_03_juntar_de_3() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_new = "hola\ncomo\nnew1\nnew2\nnew3\nnew4\nestas\nnew5\n".to_string();

        let diff_base_branch = Diff::new(str_base.clone(), str_new);
        let diff_sin_consec = juntar_consecutivos(diff_base_branch);

        let diff_esperado = vec![
            (2, true, "new1\nnew2\nnew3\nnew4".to_string()),
            (7, true, "new5".to_string()),
        ];

        assert_eq!(diff_sin_consec.lineas, diff_esperado);
    }

    #[test]
    fn juntar_consec_test_04_desde_inicio_mismo_len() {
        let str_base = "hola\ncomo\nestas\n".to_string();
        let str_new = "new1\nnew2\nnew3".to_string();

        let diff_base_branch = Diff::new(str_base.clone(), str_new);
        let diff_sin_consec = juntar_consecutivos(diff_base_branch);

        let diff_esperado = vec![
            (0, false, "hola".to_string()),
            (0, true, "new1\nnew2\nnew3".to_string()),
            (1, false, "como".to_string()),
            (2, false, "estas".to_string()),
        ];

        assert_eq!(diff_sin_consec.lineas, diff_esperado);
    }
}

#[cfg(test)]
mod merge_con_archivos {
    // use crate::commands::commands;
    // use super::*;
}

#[cfg(test)]
mod check_ignore_tests {
    use std::{fs, path::Path};

    use serial_test::serial;

    use crate::commands::commands_fn;

    use super::*;

    #[serial]
    #[test]
    fn test01_check_ignore_lee_correctamente_una_linea() {
        let path = Path::new(&"cliente");
        if path.exists() {
            fs::remove_dir_all(path).unwrap();
        }
        file_manager::create_directory(&"cliente".to_string()).unwrap();
        let cliente = "cliente".to_string();
        let flags = vec!["repo_ignore".to_string()];
        commands_fn::init(flags, cliente.clone()).unwrap();
        file_manager::write_file(
            "cliente/repo_ignore/gitrignore".to_string(),
            "/target\n".to_string(),
        )
        .unwrap();
        let paths = vec!["cliente/repo_ignore/target".to_string()];
        let vec_match = vec!["cliente/repo_ignore/target".to_string()];
        assert_eq!(check_ignore_(paths, cliente).unwrap(), vec_match);
        fs::remove_dir_all(path).unwrap();
    }

    #[serial]
    #[test]
    fn test02_check_ignore_lee_correctamente_varias_lineas_desde_archivo() {
        let path = Path::new(&"cliente");
        if path.exists() {
            fs::remove_dir_all(path).unwrap();
        }
        file_manager::create_directory(&"cliente".to_string()).unwrap();
        let cliente = "cliente".to_string();
        let flags = vec!["repo_ignore".to_string()];
        commands_fn::init(flags, cliente.clone()).unwrap();
        file_manager::write_file(
            "cliente/repo_ignore/gitrignore".to_string(),
            "/target\n/target2\n".to_string(),
        )
        .unwrap();
        let paths = vec![
            "cliente/repo_ignore/target".to_string(),
            "cliente/repo_ignore/target2".to_string(),
        ];
        let vec_match = vec![
            "cliente/repo_ignore/target".to_string(),
            "cliente/repo_ignore/target2".to_string(),
        ];
        assert_eq!(check_ignore_(paths, cliente).unwrap(), vec_match);
        fs::remove_dir_all(path).unwrap();
    }

    #[serial]
    #[test]
    fn test03_add_ignora_archivos_en_gitingore() {
        let path = Path::new(&"cliente");
        if path.exists() {
            fs::remove_dir_all(path).unwrap();
        }
        file_manager::create_directory(&"cliente".to_string()).unwrap();
        let cliente = "cliente".to_string();
        let flags = vec!["repo_ignore".to_string()];
        commands_fn::init(flags, cliente.clone()).unwrap();
        file_manager::write_file(
            "cliente/repo_ignore/gitrignore".to_string(),
            "/file1\n/folder/file3\n".to_string(),
        )
        .unwrap();
        file_manager::write_file("cliente/repo_ignore/file1".to_string(), "file1".to_string())
            .unwrap();
        file_manager::write_file("cliente/repo_ignore/file2".to_string(), "file2".to_string())
            .unwrap();

        file_manager::create_directory(&"cliente/repo_ignore/folder".to_string()).unwrap();

        file_manager::write_file(
            "cliente/repo_ignore/folder/file3".to_string(),
            "file3".to_string(),
        )
        .unwrap();
        file_manager::write_file(
            "cliente/repo_ignore/folder/file4".to_string(),
            "file3".to_string(),
        )
        .unwrap();

        commands_fn::add(vec![".".to_string()], cliente.clone()).unwrap();
        let index = file_manager::read_index(cliente.clone()).unwrap();
        assert!(!index.contains(&"cliente/repo_ignore/file1".to_string()));
        assert!(!index.contains(&"cliente/repo_ignore/folder/file3".to_string()));
        fs::remove_dir_all(path).unwrap();
    }
}


