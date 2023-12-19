use crate::file_manager::{commit_log, get_branches, get_refs_ids};
use crate::file_manager::{
    delete_tag, get_current_commit, get_current_repo, update_working_directory,
};
use crate::git_transport::ref_discovery::{self, check_push};
use crate::{file_manager, gitr_errors::GitrError};
use std::path::Path;

use super::command_utils::{self, *};

/***************************
 ***************************
 *      COMMANDS
 **************************
 **************************/

//Create an empty Gitr repository
pub fn init(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    // init <name-of-new-repo>
    if flags.is_empty() || flags.len() > 1 {
        return Err(GitrError::InvalidArgumentError(
            flags.join(" "),
            "init <new_repo_name>".to_string(),
        ));
    }
    file_manager::init_repository(&(cliente.clone() + "/" + &flags[0]))?;
    file_manager::update_current_repo(&flags[0], cliente)?;
    println!("Initialized empty Gitr repository");
    Ok(())
}

/// Computes the object ID value for an object with the contents of the named file
pub fn hash_object(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    // hash-object -w <file>
    // hash-object <file>
    if flags.len() != 1 && flags.len() != 2 {
        return Err(GitrError::InvalidArgumentError(
            flags.join(" "),
            "hash-object [<-w>] <file>".to_string(),
        ));
    }
    let mut file_path = String::new();
    let mut write = false;
    if flags.len() == 1 {
        file_path = flags[0].clone();
    }
    if flags.len() == 2 && flags[0] == "-w" {
        file_path = flags[1].clone();
        write = true;
    }
    println!("{}", get_object_hash(cliente, &mut file_path, write)?);
    Ok(())
}

pub fn cat_file(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    //cat-file -p <object-hash>
    //cat-file -t <object-hash>
    //cat-file -s <object-hash>
    if flags.len() != 2 {
        let flags_str = flags.join(" ");
        return Err(GitrError::InvalidArgumentError(
            flags_str,
            "cat-file <[-t/-s/-p]> <object hash>".to_string(),
        ));
    }
    let data_to_print = _cat_file(flags, cliente)?;
    println!("{}", data_to_print);

    Ok(())
}

//Add file contents to the index
pub fn add(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    // add <file-name>
    // add .
    if flags.len() != 1 {
        return Err(GitrError::InvalidArgumentError(
            flags.join(" "),
            "add <[file/.]>".to_string(),
        ));
    }
    update_index_before_add(cliente.clone())?;
    add_files_command(flags[0].clone(), cliente)?;

    Ok(())
}
//Remove files from the index
pub fn rm(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    //rm <file>
    if flags.len() != 1 {
        return Err(GitrError::InvalidArgumentError(
            flags.join(" "),
            "rm <file>".to_string(),
        ));
    }
    let removed = rm_from_index(&flags[0], cliente)?;
    if removed {
        println!("rm '{}'", flags[0]);
    } else {
        println!("Error: file not found");
    }
    Ok(())
}

//Record changes to the repository
pub fn commit(flags: Vec<String>, second_parent: String, cliente: String) -> Result<(), GitrError> {
    //commit -m <message-of-commit>
    if flags[0] != "-m" || flags.len() < 2 {
        return Err(GitrError::InvalidArgumentError(
            flags.join(" "),
            "commit -m <commit_message>".to_string(),
        ));
    }
    let index_path = file_manager::get_current_repo(cliente.clone())?.to_string() + "/gitr/index";
    if !Path::new(&index_path).exists() {
        return status(flags, cliente.clone());
    }
    let (not_staged, _, _) = get_untracked_notstaged_files(cliente.clone())?;
    let (new, mut modified) = get_tobe_commited_files(&not_staged, cliente.clone())?;
    let mut to_be_commited = new;
    to_be_commited.append(&mut modified);
    if to_be_commited.is_empty() {
        println!("nothing to commit, working tree clean");
        return Ok(());
    }
    if flags[1].starts_with('\"') {
        let message = &flags[1..];
        let message = message.join(" ");
        if !message.chars().any(|c| c != ' ' && c != '\"') {
            return Err(GitrError::InvalidArgumentError(
                flags.join(" "),
                "commit -m \"commit_message\"".to_string(),
            ));
        }
        get_tree_entries(message.to_string(), second_parent, cliente.clone())?;
        print_commit_confirmation(message, cliente.clone())?;
        Ok(())
    } else {
        Err(GitrError::InvalidArgumentError(
            flags.join(" "),
            "commit -m \"commit_message\"".to_string(),
        ))
    }
}

// Switch branches or restore working tree files
pub fn checkout(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    if flags.is_empty() || flags.len() > 2 || (flags.len() == 2 && flags[0] != "-b") {
        return Err(GitrError::InvalidArgumentError(
            flags.join(" "),
            "checkout <branch>".to_string(),
        ));
    }
    commit_existing(cliente.clone())?;
    let branch_to_checkout = get_branch_to_checkout(flags.clone(), cliente.clone())?;
    let current_commit = file_manager::get_commit(branch_to_checkout.clone(), cliente.clone())?;
    file_manager::update_working_directory(current_commit, cliente.clone())?;
    let path_head = format!("refs/heads/{}", branch_to_checkout);
    file_manager::update_head(&path_head, cliente.clone())?;

    Ok(())
}

//Show commit logs
pub fn log(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    // log
    commit_existing(cliente.clone())?;
    if flags.is_empty() {
        let log_res = commit_log("-1".to_string(), cliente.clone())?;
        print!("{}", log_res);
    }
    if flags.len() == 2 && flags[0] == "-n" && flags[1].parse::<usize>().is_ok() {
        let log_res = commit_log(flags[1].to_string(), cliente.clone())?;
        print!("{}", log_res);
    }
    Ok(())
}

// List, create, or delete branches
pub fn branch(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    //branch -m <origin_branch> <destination-branch>
    //branch
    //branch -d <branch-to-delete>
    //branch -l
    //branch <new-branch-name>
    if flags.is_empty()
        || (flags.len() == 1 && flags[0] == "-l")
        || (flags.len() == 1 && flags[0] == "--list")
    {
        println!("{}", print_branches(cliente.clone())?);
        return Ok(());
    }
    commit_existing(cliente.clone())?;
    if flags.len() == 2 && flags[0] == "-d" {
        branch_delete_flag(flags[1].clone(), cliente.clone())?;
    }
    if flags.len() == 3 && flags[0] == "-m" {
        branch_move_flag(flags[1].clone(), flags[2].clone(), cliente.clone())?;
    }
    if flags.len() == 1 && flags[0] != "-l" && flags[0] != "--list" {
        branch_newbranch_flag(flags[0].clone(), cliente.clone())?;
    }
    Ok(())
}

pub fn ls_files(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    //ls-files --stage
    if flags.is_empty() || flags[0] == "--cached" || flags[0] == "-c" {
        let ls_files_res = get_ls_files_cached(cliente.clone())?;
        print!("{}", ls_files_res);
        return Ok(());
    }
    if flags[0] == "--stage" {
        let res_output = file_manager::read_index(cliente.clone())?;
        println!("{}", res_output);
        return Ok(());
    }
    if flags[0] == "--deleted" {
        let res_output = get_ls_files_deleted_modified(true, cliente.clone())?;
        print!("{}", res_output);
        return Ok(());
    }
    if flags[0] == "--modified" {
        let res_output = get_ls_files_deleted_modified(false, cliente.clone())?;
        print!("{}", res_output);
        return Ok(());
    }
    Err(GitrError::InvalidArgumentError(
        flags.join(" "),
        "ls-files --stage".to_string(),
    ))
}

pub fn clone(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    init(vec![flags[1].clone()], cliente.clone())?;
    remote(vec![flags[0].clone()], cliente.clone())?;

    pullear(vec![], true, cliente)?;
    Ok(())
}

// Show the working tree status
pub fn status(_flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    let status = get_status(cliente.clone())?;
    println!("{}", status);
    Ok(())
}

pub fn tag(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    if flags.is_empty() || (flags.len() == 1 && flags[0] == "-l") {
        println!("{}", get_tags_str(cliente.clone())?);
        return Ok(());
    }
    if flags.len() >= 4 && flags[0] == "-a" && flags[2] == "-m" {
        let mut message = "".to_string();
        if flags[3].starts_with('\"') {
            let aux = &flags[3..];
            message = aux.join(" ");
            if !message.chars().any(|c| c != ' ' && c != '\"') {
                return Err(GitrError::InvalidArgumentError(
                    flags.join(" "),
                    "tag -a <tag-name> -m \"tag-message\"".to_string(),
                ));
            }
        }
        create_annotated_tag(flags[1].clone(), message.clone(), cliente.clone())?;
        return Ok(());
    }
    if flags.len() == 1 && flags[0] != "-l" {
        create_lightweight_tag(flags[0].clone(), cliente.clone())?;
        return Ok(());
    }
    if flags.len() == 2 && flags[0] == "-d" {
        let res = delete_tag(flags[1].clone(), cliente)?;
        println!("{}", res);
        return Ok(());
    }
    Err(GitrError::InvalidArgumentError(
        flags.join(" "),
        "tag [-l] [-a <tag-name> -m <tag-message>] <tag-name>".to_string(),
    ))
}

pub fn fetch(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    pullear(flags, false, cliente)
}

pub fn merge(_flags: Vec<String>, cliente: String) -> Result<(bool, String, Vec<String>), GitrError> {
    if _flags.is_empty() {
        return Err(GitrError::InvalidArgumentError(
            _flags.join(" "),
            "merge <branch-name>".to_string(),
        ));
    }
    
    let branch_name = _flags[0].clone();

    let origin_name = file_manager::get_head(cliente.clone())?
        .split('/')
        .collect::<Vec<&str>>()[2]
        .to_string();

    match merge_(origin_name, branch_name, cliente.clone()) {
        Ok((hubo_conflict_res, branch_hash_res, archivos_conflict_res)) => Ok((hubo_conflict_res, branch_hash_res, archivos_conflict_res)),
        Err(e) => Err(e),
    }
}

pub fn merge_(origin_name: String, branch_name: String, cliente: String) -> Result<(bool, String, Vec<String>), GitrError> {
    let mut hubo_conflict = false;
    let mut archivos_conflict = vec![];
    
    let branch_commits = branch_commits_list(branch_name.clone(), cliente.clone())?;
    let origin_commits = branch_commits_list(origin_name.clone(), cliente.clone())?;

    for commit in branch_commits.clone() {
        if origin_commits.contains(&commit) {
            if commit == origin_commits[0] {
                println!(
                    "Updating {}..{}",
                    &origin_commits[0][..7],
                    &branch_commits[0][..7]
                );
                println!("Fast-forward");
                fast_forward_merge(branch_name.clone(), cliente.clone())?;
                break;
            }
            (hubo_conflict, archivos_conflict) = command_utils::three_way_merge(
                commit,
                origin_commits[0].clone(),
                branch_commits[0].clone(),
                cliente.clone(),
            )?;
            if !hubo_conflict {
                println!("no hubo conflictos");
                if cliente.contains('/'){
                    return Ok((hubo_conflict, branch_commits[0].clone(), archivos_conflict));
                }
                add(vec![".".to_string()], cliente.clone())?;
                println!("no fallo en el add.");

                command_utils::create_merge_commit(
                    branch_name.clone(),
                    branch_commits[0].clone(),
                    cliente,
                )?;
            }
            break;
        }
    }
    Ok((hubo_conflict, branch_commits[0].clone(), archivos_conflict))
}

pub fn remote(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    if flags.is_empty() {
        let remote = file_manager::read_file(get_current_repo(cliente.clone())? + "/gitr/remote")?;
        println!("remote: {}", remote);
    } else {
        let parts: Vec<&str> = flags[0].split('/').collect();
        if parts.len() != 2 {
            return Err(GitrError::InvalidArgumentError(
                flags.join(" "),
                "remote <remote-url>/<remote-name>".to_string(),
            ));
        }
        file_manager::write_file(
            get_current_repo(cliente.clone())? + "/gitr/remote",
            flags[0].clone(),
        )?;
    }
    Ok(())
}

fn pullear(
    flags: Vec<String>,
    actualizar_work_dir: bool,
    cliente: String,
) -> Result<(), GitrError> {
    if !flags.is_empty() {
        return Err(GitrError::InvalidArgumentError(
            flags.join(" "),
            "pull <no-args>".to_string(),
        ));
    }

    // ########## HANDSHAKE ##########
    let mut stream = handshake("git-upload-pack".to_string(), cliente.clone())?;

    //  ########## REFERENCE DISCOVERY ##########
    let hash_n_references = protocol_reference_discovery(&mut stream)?;

    // ########## WANTS N HAVES ##########
    let pkt_needed =
        protocol_wants_n_haves(hash_n_references.clone(), &mut stream, cliente.clone())?;
    // ########## PACKFILE ##########
    if pkt_needed {
        pull_packfile(&mut stream, cliente.clone())?;
    }
    if actualizar_work_dir {
        file_manager::update_client_refs(
            hash_n_references.clone(),
            file_manager::get_current_repo(cliente.clone())?,
            cliente.clone(),
        )?;
        update_working_directory(get_current_commit(cliente.clone())?, cliente.clone())?;
    }

    Ok(())
}

pub fn pull(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    pullear(flags, true, cliente)
}

pub fn push(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    if !flags.is_empty() {
        return Err(GitrError::InvalidArgumentError(
            flags.join(" "),
            "push <no-args>".to_string(),
        ));
    }
    // ########## HANDSHAKE ##########
    let mut stream = handshake("git-receive-pack".to_string(), cliente.clone())?;

    //  ########## REFERENCE DISCOVERY ##########
    let hash_n_references = protocol_reference_discovery(&mut stream)?;
    // ########## REFERENCE UPDATE REQUEST ##########
    match check_push(hash_n_references.clone(), get_refs_ids("heads", cliente.clone())?,get_branches(cliente.clone())?, cliente.clone()) {
        Ok(_) => {}
        Err(e) => {
            match std::io::Write::write(&mut stream, "0000".as_bytes()) {
                Ok(_) => {}
                Err(_) => {
                    return Err(GitrError::ConnectionError);
                }
            }
            return Err(e);
        }
    }
    let (pkt_needed, pkt_ids) =
        reference_update_request(&mut stream, hash_n_references.clone(), cliente.clone())?;

    // ########## PACKFILE ##########
    if pkt_needed {
        println!("pushing packfile");
        push_packfile(&mut stream, pkt_ids, hash_n_references, cliente)?;
    }
    Ok(())
}

pub fn show_ref(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    if flags.len() > 1 || (flags.len() == 1 && flags[0] != "--head") {
        return Err(GitrError::InvalidArgumentError(
            flags.join(" "),
            "show-ref [--head]".to_string(),
        ));
    }
    let refs = match ref_discovery::ref_discovery(&(get_current_repo(cliente.clone())? + "/gitr")) {
        Ok(refs) => refs.0,
        Err(e) => {
            println!("Error: {}", e);
            return Ok(());
        }
    };
    let refs = refs.strip_suffix("0000").unwrap_or(&refs);
    let mut head = true;
    for refer in refs.lines() {
        let r = refer.split_at(4).1;
        if head {
            if flags.contains(&"--head".to_string()){
                println!("{}", r);
            }
            head = false;
            continue;
        }
        println!("{}", r);
    }
    Ok(())
}

pub fn ls_tree(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    if flags.is_empty() {
        return Err(GitrError::InvalidArgumentError(
            flags.join(" "),
            "ls-tree [options] <tree-hash>".to_string(),
        ));
    }
    command_utils::_ls_tree(flags, "".to_string(), cliente)?;
    Ok(())
}

pub fn list_repos(cliente: String) {
    println!("{:?}", file_manager::get_repos(cliente.clone()));
}

pub fn go_to_repo(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    if flags.len() != 1 {
        return Err(GitrError::InvalidArgumentError(
            flags.join(" "),
            "go-to-repo <repo>".to_string(),
        ));
    }
    let new_repo = flags[0].clone();
    let existing_repos = file_manager::get_repos(cliente.clone());
    if existing_repos.contains(&new_repo) {
        file_manager::update_current_repo(&new_repo, cliente)?;
    } else {
        println!("Error: repository '{}' does not exist", new_repo);
    }
    Ok(())
}

pub fn print_current_repo(cliente: String) -> Result<(), GitrError> {
    let repo = file_manager::get_current_repo(cliente.clone())?;
    println!("working on repo: {}", repo);
    Ok(())
}

pub fn echo(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    if flags.is_empty() {
        return Err(GitrError::InvalidArgumentError(
            flags.join(" "),
            "echo <string> > <file>".to_string(),
        ));
    }

    let mut texto = String::new();

    let mut hay_separador = false;
    for palabra in flags.iter() {
        if palabra == ">" {
            hay_separador = true;
            break;
        }
        texto.push_str(palabra);
        texto.push(' ');
    }
    texto = texto.trim_end().to_string();
    if !hay_separador {
        return Err(GitrError::InvalidArgumentError(
            flags.join(" "),
            "echo <string> > <file>".to_string(),
        ));
    }

    let repo_path = file_manager::get_current_repo(cliente.clone())?.to_string();

    let file_path = format!("{}/{}", repo_path, flags[flags.len() - 1]);
    println!("escribo \"{}\" en {}", texto, file_path);
    file_manager::write_file(file_path, texto)
}

pub fn rebase(flags: Vec<String>, cliente: String) -> Result<(), GitrError> {
    let origin_name = flags[0].clone();
    let branch_name = file_manager::get_head(cliente.clone())?
        .split('/')
        .collect::<Vec<&str>>()[2]
        .to_string();
    let branch_commits = command_utils::branch_commits_list(branch_name.clone(), cliente.clone())?;
    let origin_commits = command_utils::branch_commits_list(origin_name.clone(), cliente.clone())?;
    let mut to_rebase_commits: Vec<String> = vec![];
    for commit in branch_commits.clone() {
        if origin_commits.contains(&commit) {
            if commit == origin_commits[0] {
                println!("nothing to rebase");
                return Ok(());
            }
            create_rebase_commits(to_rebase_commits, origin_name, cliente.clone(), commit)?;
            break;
        }
        to_rebase_commits.push(commit);
    }
    Ok(())
}

pub fn check_ignore(paths: Vec<String>, client: String) -> Result<(), GitrError> {
    if paths.is_empty() {
        return Err(GitrError::InvalidArgumentError(
            paths.join(" "),
            "check-ignore <path>".to_string(),
        ));
    }
    let repo = file_manager::get_current_repo(client.clone())?;
    let mut full_paths = vec![];
    for linea in paths {
        full_paths.push(repo.to_owned() + linea.as_str());
    }
    match command_utils::check_ignore_(full_paths, client) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
