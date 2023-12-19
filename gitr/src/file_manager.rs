use std::collections::HashMap;

use crate::commands::command_utils::flate2compress;
use crate::gitr_errors::GitrError;
use crate::objects::pull_request::PullRequest;
use crate::{file_manager, logger};
use std::fs;
use std::fs::{File, OpenOptions, ReadDir};
use std::io::{prelude::*, Bytes};
use std::path::Path;

use chrono::{FixedOffset, TimeZone, Utc};
use flate2::read::ZlibDecoder;

/***************************
 ***************************
 *      FS FUNCTIONS
 **************************
 **************************/

/// Reads a file and returns the content as String
/// On Error returns a FileReadError
pub fn read_file(path: String) -> Result<String, GitrError> {
    let log_msg = format!("reading data from: {}", path);
    logger::log_file_operation(log_msg)?;
    match fs::read_to_string(path.clone()) {
        Ok(data) => Ok(data),
        Err(_) => {
            logger::log_error(format!("No se pudo leer: {}", path))?;
            Err(GitrError::FileReadError(path))
        }
    }
}

//receives a path of a repo and returns a vector of paths with all files outside gitr (only paths to files, not dirs)
pub fn visit_dirs(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.ends_with("gitr") {
                    continue;
                }
                let mut subfiles = visit_dirs(&path);
                files.append(&mut subfiles);
            } else if let Some(path_str) = path.to_str() {
                files.push(path_str.to_string());
            }
        }
    }
    files
}

// Writes a file with the given text
pub fn write_file(path: String, text: String) -> Result<(), GitrError> {
    let log_msg = format!("writing data to: {}", path);
    logger::log_file_operation(log_msg)?;
    let mut archivo = match File::create(&path) {
        Ok(archivo) => archivo,
        Err(_) => return Err(GitrError::FileCreationError(path)),
    };
    match archivo.write_all(text.as_bytes()) {
        Ok(_) => Ok(()),
        Err(_) => Err(GitrError::FileWriteError(path)),
    }
}

//Append text to a file (used in logger)
pub fn append_to_file(path: String, text: String) -> Result<(), GitrError> {
    let mut file = match OpenOptions::new().write(true).append(true).open(&path) {
        Ok(file) => file,
        Err(_) => return Err(GitrError::FileWriteError(path)),
    };
    match writeln!(file, "{}", text) {
        Ok(_) => Ok(()),
        Err(_) => Err(GitrError::FileWriteError(path)),
    }
}

/// Creates a directory in the current path
/// On Error returns a AlreadyInitialized
pub fn create_directory(path: &String) -> Result<(), GitrError> {
    let log_msg = format!("creating dir: {}", path);
    logger::log_file_operation(log_msg)?;
    match fs::create_dir(path) {
        Ok(_) => Ok(()),
        Err(_) => Err(GitrError::AlreadyInitialized),
    }
}

//delete all files without gitr
pub fn delete_all_files(cliente: String) -> Result<(), GitrError> {
    let repo = get_current_repo(cliente.clone())?;
    let _ = fs::remove_file(repo.clone() + "/gitr/index");
    let path = Path::new(&repo);
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if entry.file_name() != "gitr"
                && entry.file_name() != ".git"
                && entry.file_name() != "gitrignore"
            {
                if entry.path().is_file() {
                    match fs::remove_file(entry.path()) {
                        Ok(_) => continue,
                        Err(_) => {
                            return Err(GitrError::FileWriteError(
                                entry.path().display().to_string(),
                            ))
                        }
                    };
                }
                match fs::remove_dir_all(entry.path()) {
                    Ok(_) => (),
                    Err(_) => {
                        return Err(GitrError::FileWriteError(
                            entry.path().display().to_string(),
                        ))
                    }
                };
            }
        }
    }
    Ok(())
}

//updates index and adds files from three-way merge that don't have conflicts
pub fn add_new_files_from_merge(
    origin_hashmap: HashMap<String, String>,
    branch_hashmap: HashMap<String, String>,
    cliente: String,
) -> Result<(), GitrError> {
    for (path, hash) in branch_hashmap.iter() {
        if !origin_hashmap.contains_key(path) {
            file_manager::add_to_index(path, hash, cliente.clone())?;
            if let Some(parent) = std::path::Path::new(&path).parent() {
                match fs::create_dir_all(parent) {
                    Ok(_) => (),
                    Err(_) => return Err(GitrError::FileWriteError(parent.display().to_string())),
                };
            };
            let raw_data = read_file_data_from_blob_hash(hash.to_string(), cliente.clone())?;
            write_file(path.to_string(), raw_data)?;
        }
    }
    Ok(())
}

/***************************
 ***************************
 *      GIT OBJECTS
 **************************
 *************************
 * reading
 * writing
 * others
 */

// ***reading***

//receives a path to a compressed object and return its raw data in UTF8 format
fn read_compressed_file(path: &str) -> Result<Vec<u8>, GitrError> {
    let log_msg = format!("reading data from: {}", path);
    logger::log_file_operation(log_msg)?;
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return Err(GitrError::FileReadError(path.to_string())),
    };
    let mut decoder = ZlibDecoder::new(file);
    let mut buffer = Vec::new();
    match decoder.read_to_end(&mut buffer) {
        Ok(_) => Ok(buffer.clone()),
        Err(_) => {
            println!("error");
            Err(GitrError::FileReadError(path.to_string()))
        }
    }
}

//reads and object and returns raw data
pub fn read_object(object: &String, path: String, add_gitr: bool) -> Result<String, GitrError> {
    let path = parse_object_hash(object, path, add_gitr)?;
    let bytes = deflate_file(path.clone())?;
    let object_data: Vec<u8> = get_object_data_with_bytes(bytes)?;
    let first_byte = object_data[0];
    if first_byte as char == 't' {
        let tree_data = match read_tree_file(object_data) {
            Ok(data) => data,
            Err(_) => return Err(GitrError::FileReadError(path)),
        };
        return Ok(tree_data);
    }
    if first_byte as char == 'b' || first_byte as char == 'c' {
        let mut buffer = String::new();
        for byte in object_data {
            buffer.push(byte as char);
        }
        return Ok(buffer);
    }
    Err(GitrError::FileReadError(
        "No se pudo leer el objeto, bytes invalidos".to_string(),
    ))
}

// receives an blob hash and returns its raw data without header. Calls read_object() and parses. Error if not a blob.
pub fn read_file_data_from_blob_hash(hash: String, cliente: String) -> Result<String, GitrError> {
    let object_raw_data = read_object(
        &hash,
        file_manager::get_current_repo(cliente.clone())?,
        true,
    )?;
    let (header, raw_data) = match object_raw_data.split_once('\0') {
        Some((header, raw_data)) => (header, raw_data),
        None => {
            println!("Error: invalid object type");
            return Err(GitrError::FileReadError(hash));
        }
    };

    if !header.starts_with("blob") {
        println!("Error: invalid object type");
        return Err(GitrError::FileReadError(hash));
    }

    Ok(raw_data.to_string())
}

// auxiliar function of read_object().
fn get_object_data_with_bytes(bytes: Bytes<ZlibDecoder<File>>) -> Result<Vec<u8>, GitrError> {
    let mut object_data: Vec<u8> = Vec::new();
    for byte in bytes {
        let byte = match byte {
            Ok(byte) => byte,
            Err(_) => return Err(GitrError::CompressionError),
        };
        object_data.push(byte);
    }
    Ok(object_data)
}

//auxiliar for read_object(). Receives raw data and returns a String with readable data.
pub fn read_tree_file(data: Vec<u8>) -> Result<String, GitrError> {
    let mut header_buffer = String::new();
    let mut data_starting_index = 0;
    for byte in data.clone() {
        if byte == 0 {
            data_starting_index += 1;
            break;
        }
        header_buffer.push(byte as char);
        data_starting_index += 1;
    }
    let entries_buffer = get_entries_buffer_for_readtree(data, data_starting_index);
    Ok(header_buffer + "\0" + &entries_buffer)
}

//auxiliar for read_tree_file()
fn get_entries_buffer_for_readtree(data: Vec<u8>, data_starting_index: usize) -> String {
    let mut entries_buffer = String::new();
    let mut convert_to_hexa = false;
    let mut hexa_iters = 0;
    for byte in data[data_starting_index..].iter() {
        if hexa_iters == 20 {
            convert_to_hexa = false;
            hexa_iters = 0;
            entries_buffer.push('\n');
        }
        if *byte == 0 && !convert_to_hexa {
            entries_buffer.push('\0');
            convert_to_hexa = true;
            continue;
        }
        if convert_to_hexa {
            hexa_iters += 1;
            entries_buffer.push_str(&format!("{:02x}", byte));
        } else {
            entries_buffer.push(*byte as char);
        }
    }
    entries_buffer
}
// ***writing***

/// A diferencia de write_file, esta funcion recibe un vector de bytes
/// como data, y lo escribe en el archivo de path.
pub fn write_compressed_data(path: &str, data: &[u8]) -> Result<(), GitrError> {
    let log_msg = format!("writing data to: {}", path);
    logger::log_file_operation(log_msg)?;
    match File::create(path) {
        Ok(file) => file,
        Err(_) => return Err(GitrError::FileCreationError(path.to_string())),
    };

    match fs::write(path, data) {
        Ok(_) => Ok(()),
        Err(_) => Err(GitrError::FileCreationError(path.to_string())),
    }
}

pub fn get_remote(cliente: String) -> Result<String, GitrError> {
    let repo = get_current_repo(cliente)?;
    let path = repo + "/gitr/" + "remote";
    let remote = read_file(path)?;
    Ok(remote)
}

///receive compressed raw data from a file with his hash and write it in the objects folder
pub fn write_object(data: Vec<u8>, hashed_name: String, cliente: String) -> Result<(), GitrError> {
    let log_msg = format!("writing object {}", hashed_name);
    logger::log_file_operation(log_msg)?;

    let folder_name = hashed_name[0..2].to_string();
    let file_name = hashed_name[2..].to_string();
    let repo = get_current_repo(cliente.clone())?;
    let dir = repo + "/gitr/objects/";
    let folder_dir = dir.clone() + &folder_name;

    if fs::metadata(&folder_dir).is_err() {
        create_directory(&folder_dir)?;
    }
    write_compressed_data(&(folder_dir.clone() + "/" + &file_name), &data)?;
    Ok(())
}

// ***others***

//receives a path to a file and returns the decompressed data of the file content.
fn deflate_file(path: String) -> Result<Bytes<ZlibDecoder<File>>, GitrError> {
    let file = match File::open(&path) {
        Ok(file) => file,
        Err(_) => return Err(GitrError::FileReadError(path.to_string())),
    };
    let decoder = ZlibDecoder::new(file);

    let bytes = decoder.bytes();
    Ok(bytes)
}

// Le das un hash de objeto, se fija si existe y te devuelve el path completo de ese object
//podríamos recibir el path aca así es una funcion sola
//además la funcion _w_path no necesita el gitr y el del cliente si
fn parse_object_hash(object: &String, path: String, mut add_gitr: bool) -> Result<String, GitrError> {
    if object.len() < 3 {
        return Err(GitrError::ObjectNotFound(object.clone()));
    }
    let folder_name = object[0..2].to_string();
    let file_name = object[2..].to_string();

    let mut repo = path.clone();
    if path.starts_with("server") {
        add_gitr = false;
    }
    if add_gitr {
        repo += "/gitr";
    }
    let dir = repo + "/objects/";
    let folder_dir = dir.clone() + &folder_name;
    let path = dir + &folder_name + "/" + &file_name;
    if fs::metadata(folder_dir).is_err() {
        return Err(GitrError::ObjectNotFound(object.clone()));
    }
    if fs::metadata(&path).is_err() {
        return Err(GitrError::ObjectNotFound(object.clone()));
    }
    Ok(path)
}

/***************************
 ***************************
 *      GIT FILES
 **************************
 **************************/

//creates all necessary folder for the repo
pub fn init_repository(name: &String) -> Result<(), GitrError> {
    create_directory(name)?;
    create_directory(&(name.clone() + "/gitr"))?;
    create_directory(&(name.clone() + "/gitr/objects"))?;
    create_directory(&(name.clone() + "/gitr/refs"))?;
    create_directory(&(name.clone() + "/gitr/refs/heads"))?;
    create_directory(&(name.clone() + "/gitr/refs/remotes"))?;
    create_directory(&(name.clone() + "/gitr/refs/tags"))?;
    create_directory(&(name.clone() + "/gitr/refs/remotes/daemon"))?;
    write_file(
        name.clone() + "/gitr/HEAD",
        "ref: refs/heads/master".to_string(),
    )?;
    write_file(name.clone() + "/gitr/remote", "".to_string())?;
    write_file(name.clone() + "/gitrignore", "".to_string())?;
    Ok(())
}

pub fn get_current_repo(cliente: String) -> Result<String, GitrError> {
    if cliente.contains('/') {
        return Ok(cliente );
    }
    let current_repo = read_file(cliente.clone() + "/.head_repo")?;
    Ok(cliente + "/" + &current_repo)
}

pub fn read_index(cliente: String) -> Result<String, GitrError> {
    let repo = get_current_repo(cliente.clone())?;
    let path = repo + "/gitr/index";
    let data = match String::from_utf8(read_compressed_file(&path)?) {
        Ok(data) => data,
        Err(_) => return Err(GitrError::FileReadError(path)),
    };
    Ok(data)
}

//receives a blob's path and hash, and adds it to the index file
pub fn add_to_index(path: &String, hash: &String, cliente: String) -> Result<(), GitrError> {
    let mut index;
    let repo = get_current_repo(cliente.clone())?;
    let new_blob = format!("100644 {} 0 {}", hash, path);
    let dir = repo + "/gitr/index";
    if fs::metadata(dir.clone()).is_err() {
        let _ = write_file(dir.clone(), String::from(""));
        index = new_blob;
    } else {
        index = read_index(cliente.clone())?;
        let mut overwrited = false;
        for line in index.clone().lines() {
            let attributes = line.split(' ').collect::<Vec<&str>>();

            if attributes[3] == path {
                let log_msg = format!("adding {} to index", path);
                logger::log_action(log_msg)?;
                index = index.replace(line, &new_blob);
                overwrited = true;
                break;
            }
        }
        if !overwrited {
            index = index + "\n" + &new_blob;
        }
    }
    let compressed_index = flate2compress(index)?;
    write_compressed_data(dir.as_str(), compressed_index.as_slice())?;
    Ok(())
}

///returns the path of the head branch
pub fn get_head(cliente: String) -> Result<String, GitrError> {
    let repo = get_current_repo(cliente.clone())?;
    let mut path = repo + "/gitr/HEAD";

    if cliente.contains('/') {
        path = path.replace("/gitr/", "/")
    }

    if fs::metadata(path.clone()).is_err() {
        write_file(path.clone(), String::from("ref: refs/heads/master"))?;
        return Ok("None".to_string());
    }
    let head = read_file(path.clone())?;
    let head = head.trim_end().to_string();
    let head = head.split(' ').collect::<Vec<&str>>()[1];
    Ok(head.to_string())
}

//receives the path of the new head, updates head file
pub fn update_head(head: &String, cliente: String) -> Result<(), GitrError> {
    let repo = get_current_repo(cliente.clone())?;
    let path = repo + "/gitr/HEAD";
    write_file(path.clone(), format!("ref: {}", head))?;
    Ok(())
}

fn find_new_path(hash: String, sec_vec: Vec<(String, String)>) -> String {
    for (h, r) in sec_vec {
        if h == hash && r.clone() != "HEAD" {
            return r;
        }
    }
    "".to_string()
}

// recibe el vector de los hashes de las referencias que sacas del ref discovery, y actualiza el gitr en base a eso
pub fn update_client_refs(
    hash_n_refs: Vec<(String, String)>,
    r_path: String,
    cliente: String,
) -> Result<(), GitrError> {
    let path = r_path + "/gitr/";
    let sec_vec = hash_n_refs.clone();
    for (h, r) in hash_n_refs {
        if r.clone() == "HEAD" {
            let path_head = find_new_path(h.clone(), sec_vec.clone());
            file_manager::update_head(&path_head.replace('\\', "/"), cliente.clone())?;
            continue;
        }
        let path_ref = path.clone() + &r.replace('\\', "/"); //esto se borra?
        if let Ok(()) = file_manager::write_file(path_ref.clone(), h) {
            continue;
        }
        return Err(GitrError::FileCreationError(path_ref));
    }
    Ok(())
}

//returns a vec with all branches paths in repo
pub fn get_branches(cliente: String) -> Result<Vec<String>, GitrError> {
    let mut branches: Vec<String> = Vec::new();
    let repo = get_current_repo(cliente.clone())?;
    let dir = if cliente.contains('/') {
        repo + "/refs/heads"
    } else { 
        repo + "/gitr/refs/heads/"
    };
    let paths = match fs::read_dir(dir.clone()) {
        Ok(paths) => paths,
        Err(_) => return Err(GitrError::FileReadError(dir)),
    };
    for path in paths {
        let path = match path {
            Ok(path) => path,
            Err(_) => return Err(GitrError::FileReadError(dir)),
        };
        let path = path.path();
        let path = path.to_str();
        let path = match path {
            Some(path) => path,
            None => return Err(GitrError::FileReadError(dir)),
        };
        let path = path.split('/').collect::<Vec<&str>>();
        let path = path[path.len() - 1];
        branches.push(path.to_string());
    }
    Ok(branches)
}
pub fn get_tags(cliente: String) -> Result<Vec<String>, GitrError> {
    let mut branches: Vec<String> = Vec::new();
    let repo = get_current_repo(cliente.clone())?;
    let dir = repo + "/gitr/refs/tags";
    let paths = match fs::read_dir(dir.clone()) {
        Ok(paths) => paths,
        Err(_) => return Err(GitrError::FileReadError(dir)),
    };
    for path in paths {
        let path = match path {
            Ok(path) => path,
            Err(_) => return Err(GitrError::FileReadError(dir)),
        };
        let path = path.path();
        let path = path.to_str();
        let path = match path {
            Some(path) => path,
            None => return Err(GitrError::FileReadError(dir)),
        };
        let path = path.split('/').collect::<Vec<&str>>();
        let path = path[path.len() - 1];
        branches.push(path.to_string());
    }
    Ok(branches)
}

pub fn delete_tag(tag: String, cliente: String) -> Result<String, GitrError> {
    let repo = get_current_repo(cliente.clone())?;
    let path = format!("{}/gitr/refs/tags/{}", repo, tag);
    let hash = match read_file(path.clone()) {
        Ok(hash) => hash,
        Err(_) => return Err(GitrError::TagNonExistsError(tag)),
    };
    match fs::remove_file(path) {
        Ok(_) => (),
        Err(_) => return Err(GitrError::TagNonExistsError(tag)),
    };
    let hash_first_seven = &hash[0..7];
    let res = format!("Deleted tag '{}' (was {})", tag, hash_first_seven);
    Ok(res)
}

//delete a branch in folder refs/heads
pub fn delete_branch(branch: String, moving: bool, cliente: String) -> Result<(), GitrError> {
    let repo = get_current_repo(cliente.clone())?;
    let path = format!("{}/gitr/refs/heads/{}", repo, branch);
    let head = get_head(cliente.clone())?;
    if moving {
        let _ = fs::remove_file(path);
        return Ok(());
    }
    let current_head = repo + "/gitr/" + &head;
    if current_head == path || head == "None" {
        return Err(GitrError::DeleteCurrentBranchError(branch));
    }
    let _ = fs::remove_file(path);
    println!("Deleted branch {}", branch);
    Ok(())
}
//rename the refs/heads/branch name to the new one
pub fn move_branch(old_branch: String, new_branch: String) -> Result<(), GitrError> {
    match fs::rename(old_branch, new_branch.clone()) {
        Ok(_) => (),
        Err(_) => return Err(GitrError::FileCreationError(new_branch)),
    }
    Ok(())
}

///returns the current commit hash
pub fn get_current_commit(cliente: String) -> Result<String, GitrError> {
    let head_path = get_head(cliente.clone())?;
    if head_path == "None" {
        return Err(GitrError::NoHead);
    }
    let repo = get_current_repo(cliente.clone())?;
    let mut path = repo + "/gitr/" + &head_path;

    if cliente.contains('/') {
        path = path.replace("/gitr/", "/")
    }

    let head = read_file(path)?;
    Ok(head)
}

//receives a branch and returns its commit hash
pub fn get_commit(branch: String, cliente: String) -> Result<String, GitrError> {
    let repo = match get_current_repo(cliente.clone()) {
        Ok(repo) => repo,
        Err(_) => cliente.clone() //si no hay repo, es porque es un server
    };
    let path = format!("{}/gitr/refs/heads/{}", repo, branch);

    let commit = match read_file(path) {
        Ok(commit) => commit,
        Err(_) => {
            let path_server = format!("{}/refs/heads/{}", repo, branch);
            read_file(path_server)?
        },
    
    };
    Ok(commit)
}

//receives a path and a hash and creates a tree
pub fn create_tree(path: String, hash: String, cliente: String) -> Result<(), GitrError> {
    file_manager::create_directory(&path)?;
    let tree_raw_data = read_object(
        &hash,
        file_manager::get_current_repo(cliente.clone())?,
        true,
    )?;
    let raw_data = match tree_raw_data.split_once('\0') {
        Some((_, raw_data)) => raw_data,
        None => {
            println!("Error: invalid object type");
            return Ok(());
        }
    };
    for entry in raw_data.split('\n') {
        let object = entry.split(' ').collect::<Vec<&str>>()[0];
        if object == "100644" {
            let path_completo = path.clone() + "/" + &parse_blob_path(entry.to_string().clone());
            let hash = parse_blob_hash(entry.to_string().clone());
            create_blob(path_completo, hash, cliente.clone())?;
        } else {
            let _new_path_hash = entry.split(' ').collect::<Vec<&str>>()[1];
            let new_path = _new_path_hash.split('\0').collect::<Vec<&str>>()[0];
            let hash = _new_path_hash.split('\0').collect::<Vec<&str>>()[1];
            create_tree(
                path.clone() + "/" + new_path,
                hash.to_string(),
                cliente.clone(),
            )?;
        }
    }
    Ok(())
}
//auxiliar function for create_tree
fn parse_blob_hash(blob_entry: String) -> String {
    let _new_path_hash = blob_entry.split(' ').collect::<Vec<&str>>()[1];
    let hash = _new_path_hash.split('\0').collect::<Vec<&str>>()[1];
    hash.to_string()
}
//auxiliar function for create_tree
fn parse_blob_path(blob_entry: String) -> String {
    let _new_path_hash = blob_entry.split(' ').collect::<Vec<&str>>()[1];
    let new_path = _new_path_hash.split('\0').collect::<Vec<&str>>()[0];
    new_path.to_string()
}

//receives a path and a hash and creates a blob
pub fn create_blob(path: String, hash: String, cliente: String) -> Result<(), GitrError> {
    let new_blob = read_object(
        &(hash.to_string()),
        file_manager::get_current_repo(cliente.clone())?,
        true,
    )?;
    let new_blob_only_data = new_blob.split('\0').collect::<Vec<&str>>()[1];
    add_to_index(&path, &hash, cliente.clone())?;
    write_file(path.to_string(), new_blob_only_data.to_string())?;
    Ok(())
}

//receives a commit and updates the repo with the content of the commit
pub fn update_working_directory(commit: String, cliente: String) -> Result<(), GitrError> {
    delete_all_files(cliente.clone())?;
    let main_tree = get_main_tree(commit, cliente.clone())?;
    let tree = read_object(
        &main_tree,
        file_manager::get_current_repo(cliente.clone())?,
        true,
    )?;
    let raw_data = match tree.split_once('\0') {
        Some((_, raw_data)) => raw_data,
        None => {
            println!("Error: invalid object type");
            return Ok(());
        }
    };
    let repo = get_current_repo(cliente.clone())? + "/";
    for entry in raw_data.split('\n') {
        let object: &str = entry.split(' ').collect::<Vec<&str>>()[0];
        if object == "40000" {
            let _new_path_hash = entry.split(' ').collect::<Vec<&str>>()[1];
            let new_path = repo.clone() + _new_path_hash.split('\0').collect::<Vec<&str>>()[0];
            let hash = _new_path_hash.split('\0').collect::<Vec<&str>>()[1];
            create_tree(new_path.to_string(), hash.to_string(), cliente.clone())?;
        } else {
            let path_completo = repo.clone() + parse_blob_path(entry.to_string().clone()).as_str();
            let hash = parse_blob_hash(entry.to_string().clone());

            create_blob(path_completo, hash, cliente.clone())?;
        }
    }
    Ok(())
}

//receives a commit and returns its main tree hash
pub fn get_main_tree(commit: String, cliente: String) -> Result<String, GitrError> {
    let commit = read_object(
        &commit,
        file_manager::get_current_repo(cliente.clone())?,
        true,
    )?;
    let commit = commit.split('\n').collect::<Vec<&str>>();
    let tree_base = commit[0].split('\0').collect::<Vec<&str>>()[1];
    let tree_hash_str = tree_base.split(' ').collect::<Vec<&str>>()[1];
    Ok(tree_hash_str.to_string())
}

//receives a commit and returns its parent commit hash
pub fn get_parent_commit(commit: String, cliente: String) -> Result<Vec<String>, GitrError> {
    let add_gitr = !cliente.contains('/');
    let commit = read_object(
        &commit,
        file_manager::get_current_repo(cliente.clone())?,
        add_gitr,
    )?;
    let commit = commit.split('\n').collect::<Vec<&str>>();
    if commit[1].split(' ').collect::<Vec<&str>>()[0] != "parent" {
        return Ok(vec!["None".to_string()]);
    }

    let mut parents: Vec<String> = Vec::new();
    parents.push(commit[1].split(' ').collect::<Vec<&str>>()[1].to_string());

    if commit[2].starts_with("parent") {
        parents.push(commit[2].split(' ').collect::<Vec<&str>>()[1].to_string());
    }
    Ok(parents)
}

//receives a commit and returns its commmiter mail
pub fn get_commit_commiter_mail(commit: String, cliente: String) -> Result<String, GitrError> {
    let commit = read_object(
        &commit,
        file_manager::get_current_repo(cliente.clone())?,
        true,
    )?;
    let commit = commit.split('\n').collect::<Vec<&str>>();

    let mut idx = 3;
    if commit[3].split(' ').collect::<Vec<&str>>()[0] != "committer" {
        idx -= 1;
    }

    let author = commit[idx].split(' ').collect::<Vec<&str>>()[2];
    Ok(author.to_string())
}

//receives a commit and returns its commmiter name
pub fn get_commit_commiter(commit: String, cliente: String) -> Result<String, GitrError> {
    let commit = read_object(
        &commit,
        file_manager::get_current_repo(cliente.clone())?,
        true,
    )?;
    let commit = commit.split('\n').collect::<Vec<&str>>();

    let mut idx = 3;
    if commit[3].split(' ').collect::<Vec<&str>>()[0] != "committer" {
        idx -= 1;
    }


    let author = commit[idx].split(' ').collect::<Vec<&str>>()[1];
    Ok(author.to_string())
}

//receives a commit and returns its author
pub fn get_commit_author_mail(commit: String, cliente: String) -> Result<String, GitrError> {
    let commit = read_object(
        &commit,
        file_manager::get_current_repo(cliente.clone())?,
        true,
    )?;
    let commit = commit.split('\n').collect::<Vec<&str>>();

    let mut idx = 2;
    if commit[2].split(' ').collect::<Vec<&str>>()[0] != "author" {
        idx -= 1;
    }
    let mail = commit[idx].split(' ').collect::<Vec<&str>>()[2];
    Ok(mail.to_string())
}

//receives a commit and returns its author name
pub fn get_commit_author(commit: String, cliente: String) -> Result<String, GitrError> {
    let commit = read_object(
        &commit,
        file_manager::get_current_repo(cliente.clone())?,
        true,
    )?;
    let commit = commit.split('\n').collect::<Vec<&str>>();
    let mut idx = 2;
    if commit[1].split(' ').collect::<Vec<&str>>()[0] != "parent" {
        idx -= 1;
    } else if commit[2].starts_with("parent") {
        idx += 1;
    }
    let author = commit[idx].split(' ').collect::<Vec<&str>>()[1];
    Ok(author.to_string())
}

//receives a commit and returns its date
pub fn get_commit_date(commit: String, cliente: String) -> Result<String, GitrError> {
    let commit = read_object(
        &commit,
        file_manager::get_current_repo(cliente.clone())?,
        true,
    )?;
    let commit = commit.split('\n').collect::<Vec<&str>>();
    let mut idx = 2;
    if commit[1].split(' ').collect::<Vec<&str>>()[0] != "parent" {
        idx -= 1;
    } else if commit[2].starts_with("parent") {
        idx += 1;
    }
    let timestamp = commit[idx].split(' ').collect::<Vec<&str>>()[3];
    let timestamp_parsed = match timestamp.parse::<i64>() {
        Ok(timestamp) => timestamp,
        Err(_) => return Err(GitrError::TimeError),
    };
    let dt = Utc.timestamp_opt(timestamp_parsed, 0);
    let dt = match dt.single() {
        Some(dt) => dt,
        None => return Err(GitrError::TimeError),
    };

    let offset = FixedOffset::east_opt(-3 * 3600);
    let offset = match offset {
        Some(offset) => offset,
        None => return Err(GitrError::TimeError),
    };
    let dt = dt.with_timezone(&offset);

    let date = dt.format("%a %b %d %H:%M:%S %Y %z").to_string();
    Ok(date)
}

//receives a commit and returns its message
pub fn get_commit_message(commit: String, cliente: String) -> Result<String, GitrError> {
    let commit = read_object(
        &commit,
        file_manager::get_current_repo(cliente.clone())?,
        true,
    )?;
    let commit = commit.split('\n').collect::<Vec<&str>>();
    let mut idx = 5;
    if commit[1].split(' ').collect::<Vec<&str>>()[0] != "parent" {
        idx -= 1;
    }
    let message = commit[idx..].join("\n");
    Ok(message)
}

//updates the current repo file

pub fn update_current_repo(dir_name: &String, cliente: String) -> Result<(), GitrError> {
    write_file(cliente + "/.head_repo", dir_name.to_string())?;
    Ok(())
}

/// Devuelve vector con los ids de los commits en los heads activos
pub fn get_refs_ids(carpeta: &str, cliente: String) -> Result<Vec<String>, GitrError> {
    let mut branches: Vec<String> = Vec::new();
    let repo = get_current_repo(cliente.clone())?;
    let dir = repo + "/gitr/refs/" + carpeta;
    let paths = match fs::read_dir(dir.clone()) {
        Ok(paths) => paths,
        Err(_) => return Err(GitrError::FileReadError(dir)),
    };
    for path in paths {
        let path = match path {
            Ok(path) => path,
            Err(_) => return Err(GitrError::FileReadError(dir)),
        };
        let path = path.path();
        let path = path.to_str();
        let path = match path {
            Some(path) => path,
            None => return Err(GitrError::FileReadError(dir)),
        };
        let content = read_file(path.to_string())?;
        branches.push(content);
    }
    Ok(branches)
}

//receives a quantity and returns that number of commits from logs
pub fn commit_log(quantity: String, cliente: String) -> Result<String, GitrError> {
    let mut res: String = "".to_owned();
    let mut current_commit = get_current_commit(cliente.clone())?;
    let limit = match quantity.parse::<i32>() {
        Ok(quantity) => quantity,
        Err(_) => {
            return Err(GitrError::InvalidArgumentError(
                quantity,
                "log <quantity>".to_string(),
            ))
        }
    };
    let mut counter = 0;
    loop {
        counter += 1;
        let parents = get_parent_commit(current_commit.clone(), cliente.clone())?;
        if parents.len() == 2 {
            let parent_1 = parents[0].split_at(7).0;
            let parent_2 = parents[1].split_at(7).0;
            let format_merge = format!("Merge: {} {}\n", parent_1, parent_2);
            res.push_str(&format_merge);
        }
        let format_commit = format!("commit: {}\n", current_commit);
        res.push_str(&format_commit);
        let date = get_commit_date(current_commit.clone(), cliente.clone())?;
        let author = get_commit_author(current_commit.clone(), cliente.clone())?;
        let message = get_commit_message(current_commit.clone(), cliente.clone())?;
        res.push_str(&format!("Author: {}\n", author));
        res.push_str(&format!("Date: {}\n", date));
        res.push_str(&format!("\t{}\n\n", message));
        if parents[0] == "None" || counter == limit {
            break;
        }
        current_commit = parents[0].clone();
    }
    Ok(res.to_string())
}

//returns all repos
pub fn get_repos(cliente: String) -> Vec<String> {
    let mut repos: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir("./".to_string() + &cliente.clone()) {
        for entry in entries.flatten() {
            if entry.file_name() == "gitr"
                || entry.file_name() == "src"
                || entry.file_name() == "tests"
                || entry.file_name() == "target"
            {
                continue;
            }
            if entry.file_type().unwrap().is_dir() {
                let p = entry
                    .path()
                    .display()
                    .to_string()
                    .split('/')
                    .collect::<Vec<&str>>()[2]
                    .to_string();
                repos.push(p.to_string());
            }
        }
    }
    repos
}

//removes a file
pub fn remove_file(path: String) -> Result<(), GitrError> {
    match fs::remove_file(path.clone()) {
        Ok(_) => Ok(()),
        Err(_) => Err(GitrError::FileDeleteError(path)),
    }
}

pub fn get_all_objects_hashes(cliente: String) -> Result<Vec<String>, GitrError> {
    let mut objects: Vec<String> = Vec::new();
    let repo = get_current_repo(cliente.clone())?;
    let dir: String = repo + "/gitr/objects";
    let dir_reader = match fs::read_dir(dir.clone()) {
        Ok(l) => l,
        Err(_) => return Err(GitrError::FileReadError(dir)),
    };
    iterate_over_dirs_for_getting_objects_hashes(dir_reader, &mut objects, dir)?;
    Ok(objects)
}

//Función auxiliar de get_all_objects_hashes
fn iterate_over_dirs_for_getting_objects_hashes(
    dir_reader: ReadDir,
    objects: &mut Vec<String>,
    dir: String,
) -> Result<(), GitrError> {
    for carpeta_rs in dir_reader {
        let carpeta = match carpeta_rs {
            Ok(path) => path,
            Err(_) => return Err(GitrError::FileReadError(dir)),
        };
        let f = carpeta.file_name();
        let dir_name = f.to_str().unwrap_or("Error");
        if dir_name == "Error" {
            return Err(GitrError::FileReadError(dir));
        }
        let file_reader = match fs::read_dir(dir.clone() + "/" + dir_name) {
            Ok(l) => l,
            Err(_) => return Err(GitrError::FileReadError(dir)),
        };
        for file in file_reader {
            let file = match file {
                Ok(path) => path,
                Err(_) => return Err(GitrError::FileReadError(dir)),
            };
            let f = file.file_name();
            let file_name = f.to_str().unwrap_or("Error");
            if file_name == "Error" {
                return Err(GitrError::FileReadError(dir));
            }
            let object = dir_name.to_string() + file_name;
            objects.push(object);
        }
    }
    Ok(())
}

pub fn get_object(id: String, r_path: String) -> Result<String, GitrError> {
    let dir_path = format!("{}/objects/{}", r_path.clone(), id.split_at(2).0);
    let mut archivo = match File::open(format!("{}/{}", dir_path, id.split_at(2).1)) {
        Ok(archivo) => archivo,
        Err(_) => return Err(GitrError::FileReadError(dir_path)),
    };
    let mut contenido: Vec<u8> = Vec::new();
    if archivo.read_to_end(&mut contenido).is_err() {
        return Err(GitrError::FileReadError(dir_path));
    }
    let descomprimido = String::from_utf8_lossy(&decode(&contenido)?).to_string();
    Ok(descomprimido)
}

pub fn get_object_bytes(id: String, r_path: String) -> Result<Vec<u8>, GitrError> {
    let dir_path = format!("{}/objects/{}", r_path.clone(), id.split_at(2).0);
    let mut archivo = match File::open(format!("{}/{}", dir_path, id.split_at(2).1)) {
        Ok(archivo) => archivo,
        Err(_) => return Err(GitrError::FileReadError(dir_path)),
    };
    let mut contenido: Vec<u8> = Vec::new();
    if archivo.read_to_end(&mut contenido).is_err() {
        return Err(GitrError::FileReadError(dir_path));
    }
    decode(&contenido)
}

pub fn decode(input: &[u8]) -> Result<Vec<u8>, GitrError> {
    let mut decoder = ZlibDecoder::new(input);
    let mut decoded_data = Vec::new();
    if decoder.read_to_end(&mut decoded_data).is_err() {
        return Err(GitrError::CompressionError);
    }
    Ok(decoded_data)
}

pub fn create_pull_request(server_path: &str, pull_request: PullRequest) -> Result<(), GitrError> {
    write_file(server_path.to_string(), pull_request.to_string()?)?;
    Ok(())
}

pub fn get_pull_request(remote: &str, id: &str) -> Result<String, GitrError> {
    let path = format!("{}/pulls/{}", remote, id);
    let data = read_file(path)?;
    Ok(data)
}

pub fn get_pull_requests(mut dir: String) -> Result<Vec<PullRequest>, GitrError> {
    dir = dir.to_owned() + "/pulls";
    let mut pull_requests: Vec<PullRequest> = Vec::new();
    let paths = match fs::read_dir(dir.clone()) {
        Ok(paths) => paths,
        Err(_) => return Err(GitrError::FileReadError(dir)),
    };
    
    for path in paths {
        let path = match path {
            Ok(path) => path,
            Err(_) => return Err(GitrError::FileReadError(dir)),
        };
        let path = path.path();
        let path = path.to_str();
        let path = match path {
            Some(path) => path,
            None => return Err(GitrError::FileReadError(dir)),
        };
        let content = read_file(path.to_string())?;
        let pull_request = PullRequest::from_string(content)?;
        pull_requests.push(pull_request);
    }
    pull_requests.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(pull_requests)
}

pub fn contar_archivos_y_directorios(ruta: &str) -> Result<usize, GitrError> {
    let entradas = match fs::read_dir(ruta){
        Ok(entradas) => entradas,
        Err(_) => return Err(GitrError::FileReadError(ruta.to_string())),
    }; 
    let cuenta = entradas.count();
    Ok(cuenta)
}

pub fn pull_request_exist(path: &str) -> bool {
    let data = read_file(path.to_string());
    data.is_ok()
}