use gitr::commands::command_utils;
use gitr::commands::command_utils::*;
use std::{fs, path::Path};

use gitr::commands::commands_fn;
use gitr::file_manager;
use gitr::file_manager::*;
use gitr::gitr_errors::GitrError;
use gitr::objects::blob::Blob;
use serial_test::serial;

/*********************
      INIT TESTS
*********************/

#[test]
#[serial]
fn test_init() {
    let cliente = "cliente_init".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(vec!["test_init".to_string()], cliente.clone()).unwrap();
    let directory_path = String::from("cliente_init/test_init/gitr/");
    assert!(Path::new(&(directory_path.clone() + "objects")).is_dir());
    assert!(Path::new(&(directory_path.clone() + "refs")).is_dir());
    assert!(Path::new(&(directory_path.clone() + "refs/heads")).is_dir());
    let current_repo = get_current_repo(cliente.clone()).unwrap();
    assert_eq!("cliente_init/test_init", current_repo);
    fs::remove_dir_all(cliente.clone()).unwrap();
}

#[test]
#[serial]
fn test_init_exists() {
    let cliente = "cliente_init_2".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(vec!["test_init_exists".to_string()], cliente.clone()).unwrap();
    let res = commands_fn::init(vec!["test_init_exists".to_string()], cliente.clone());
    let error = res.unwrap_err();
    assert!(matches!(error, GitrError::AlreadyInitialized));
    fs::remove_dir_all("cliente_init_2").unwrap();
}

// /*********************
//   ADD TESTS
// *********************/
#[test]
#[serial]
fn test_add() {
    let cliente = "cliente_add".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(vec!["test_add_blob".to_string()], cliente.clone()).unwrap();
    let _ = write_file(
        (cliente.clone() + "/test_add_blob/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_add_blob/blob2").to_string(),
        "Hello, im blob 2".to_string(),
    );
    commands_fn::add(vec!["blob1".to_string()], cliente.clone()).unwrap();
    commands_fn::add(vec!["blob2".to_string()], cliente.clone()).unwrap();
    let hash1 = Blob::new("Hello, im blob 1".to_string())
        .unwrap()
        .get_hash();
    let hash2 = Blob::new("Hello, im blob 2".to_string())
        .unwrap()
        .get_hash();
    assert!(Path::new(&(cliente.clone() + "/test_add_blob/gitr/index")).is_file());
    let index = file_manager::read_index(cliente.clone()).unwrap();
    assert!(index.contains(&hash1));
    assert!(index.contains(&hash2));
    let hash1_dir = (cliente.clone() + "/test_add_blob/gitr/objects/").to_string() + &hash1[..2];
    let hash2_dir = (cliente.clone() + "/test_add_blob/gitr/objects/").to_string() + &hash2[..2];
    assert!(Path::new(&hash1_dir).is_dir());
    assert!(Path::new(&hash2_dir).is_dir());
    let hash1_file = hash1_dir.clone() + "/" + &hash1[2..];
    let hash2_file = hash2_dir.clone() + "/" + &hash2[2..];
    assert!(Path::new(&hash1_file).is_file());
    assert!(Path::new(&hash2_file).is_file());
    fs::remove_dir_all(cliente).unwrap();
}

// /*********************
//   RM TESTS
// *********************/
#[test]
#[serial]
fn test_rm() {
    let cliente = "cliente_rm".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(vec!["test_rm_blob".to_string()], cliente.clone()).unwrap();
    let _ = write_file(
        (cliente.clone() + "/test_rm_blob/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_rm_blob/blob2").to_string(),
        "Hello, im blob 2".to_string(),
    );
    commands_fn::add(vec!["blob1".to_string()], cliente.clone()).unwrap();
    commands_fn::add(vec!["blob2".to_string()], cliente.clone()).unwrap();
    let hash1 = Blob::new("Hello, im blob 1".to_string())
        .unwrap()
        .get_hash();
    let hash2 = Blob::new("Hello, im blob 2".to_string())
        .unwrap()
        .get_hash();
    assert!(Path::new(&(cliente.clone() + "/test_rm_blob/gitr/index")).is_file());
    let index = file_manager::read_index(cliente.clone()).unwrap();
    assert!(index.contains(&hash1));
    assert!(index.contains(&hash2));
    let hash1_dir = (cliente.clone() + "/test_rm_blob/gitr/objects/").to_string() + &hash1[..2];
    let hash2_dir = (cliente.clone() + "/test_rm_blob/gitr/objects/").to_string() + &hash2[..2];
    assert!(Path::new(&hash1_dir).is_dir());
    assert!(Path::new(&hash2_dir).is_dir());
    let hash1_file = hash1_dir.clone() + "/" + &hash1[2..];
    let hash2_file = hash2_dir.clone() + "/" + &hash2[2..];
    assert!(Path::new(&hash1_file).is_file());
    assert!(Path::new(&hash2_file).is_file());
    commands_fn::rm(vec!["blob1".to_string()], cliente.clone()).unwrap();
    commands_fn::rm(vec!["blob2".to_string()], cliente.clone()).unwrap();
    let index = file_manager::read_index(cliente.clone()).unwrap();
    assert!(!index.contains(&hash1));
    assert!(!index.contains(&hash2));
    fs::remove_dir_all(cliente).unwrap();
}

// /*********************
//   LS_FILES TESTS
// *********************/
#[test]
#[serial]
fn test_ls_files_returns_empty_after_init() {
    let cliente = "cliente_ls_files_1".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(vec!["test_ls_files_empty".to_string()], cliente.clone()).unwrap();
    let res = command_utils::get_ls_files_cached(cliente.clone()).unwrap();
    assert!(res.is_empty());
    fs::remove_dir_all(cliente.clone()).unwrap();
}

#[test]
#[serial]
fn test_ls_files_stage_after_adding_files() {
    let cliente = "cliente_ls_files_2".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(vec!["test_ls_files_stage".to_string()], cliente.clone()).unwrap();
    let _ = write_file(
        (cliente.clone() + "/test_ls_files_stage/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_ls_files_stage/blob2").to_string(),
        "Hello, im blob 2".to_string(),
    );
    commands_fn::add(vec!["blob1".to_string()], cliente.clone()).unwrap();
    commands_fn::add(vec!["blob2".to_string()], cliente.clone()).unwrap();
    let res = read_index(cliente.clone()).unwrap();
    let correct_res = String::from("100644 016a41a6a35d50d311286359f1a7611948a9c529 0 cliente_ls_files_2/test_ls_files_stage/blob1\n100644 18d74b139e1549bb6a96b281e6ac3a0ec9e563e8 0 cliente_ls_files_2/test_ls_files_stage/blob2");
    fs::remove_dir_all(cliente.clone()).unwrap();
    assert_eq!(res, correct_res);
}

/*********************
  TAG TESTS
*********************/
#[test]
#[serial]
fn test_tag_lightweight() {
    let cliente = "cliente_tag_lightweight".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(vec!["test_tag_lightweight".to_string()], cliente.clone()).unwrap();
    let _ = write_file(
        (cliente.clone() + "/gitrconfig").to_string(),
        ("[user]\n\tname = test\n\temail =test@gmail.com").to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_tag_lightweight/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_tag_lightweight/blob2").to_string(),
        "Hello, im blob 2".to_string(),
    );
    commands_fn::add(vec!["blob1".to_string()], cliente.clone()).unwrap();
    commands_fn::add(vec!["blob2".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"commit 1\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();
    commands_fn::tag(vec!["tag1".to_string()], cliente.clone()).unwrap();
    let res =
        file_manager::read_file(cliente.clone() + "/test_tag_lightweight/gitr/refs/tags/tag1")
            .unwrap();
    let current_commit = file_manager::get_current_commit(cliente.clone()).unwrap();
    assert_eq!(res, current_commit);
    fs::remove_dir_all(cliente.clone()).unwrap();
}

#[test]
#[serial]
fn test_tag_delete() {
    let cliente = "cliente_tag_delete".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(vec!["test_tag_delete".to_string()], cliente.clone()).unwrap();
    let _ = write_file(
        (cliente.clone() + "/gitrconfig").to_string(),
        ("[user]\n\tname = test\n\temail =test@gmail.com").to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_tag_delete/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_tag_delete/blob2").to_string(),
        "Hello, im blob 2".to_string(),
    );
    commands_fn::add(vec!["blob1".to_string()], cliente.clone()).unwrap();
    commands_fn::add(vec!["blob2".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"commit 1\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();
    commands_fn::tag(vec!["tag1".to_string()], cliente.clone()).unwrap();
    let res =
        file_manager::read_file(cliente.clone() + "/test_tag_delete/gitr/refs/tags/tag1").unwrap();
    let current_commit = file_manager::get_current_commit(cliente.clone()).unwrap();
    assert_eq!(res, current_commit);
    commands_fn::tag(vec!["-d".to_string(), "tag1".to_string()], cliente.clone()).unwrap();
    let res = file_manager::read_file(cliente.clone() + "/test_tag_delete/gitr/refs/tags/tag1");
    assert!(res.is_err());
    fs::remove_dir_all(cliente.clone()).unwrap();
}

#[test]
#[serial]
fn test_tag_annotated() {
    let cliente = "cliente_tag_annotated".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(vec!["test_tag_annotated".to_string()], cliente.clone()).unwrap();
    let _ = write_file(
        (cliente.clone() + "/gitrconfig").to_string(),
        "[user]\n\tname = test\n\temail = test@gmail.com".to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_tag_annotated/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_tag_annotated/blob2").to_string(),
        "Hello, im blob 2".to_string(),
    );
    commands_fn::add(vec!["blob1".to_string()], cliente.clone()).unwrap();
    commands_fn::add(vec!["blob2".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"commit 1\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();
    commands_fn::tag(
        vec![
            "-a".to_string(),
            "tag1".to_string(),
            "-m".to_string(),
            "\"un tag anotado\"".to_string(),
        ],
        cliente.clone(),
    )
    .unwrap();
    let res = file_manager::read_file(cliente.clone() + "/test_tag_annotated/gitr/refs/tags/tag1")
        .unwrap();
    let object = file_manager::read_object(
        &res,
        file_manager::get_current_repo(cliente.clone()).unwrap(),
        true,
    )
    .unwrap();
    let object_type = object.split(' ').collect::<Vec<&str>>()[0];
    assert_eq!(object_type, "tag");
    fs::remove_dir_all(cliente.clone()).unwrap();
}

// /*********************
//   BRANCH TESTS
// *********************/
#[test]
#[serial]
fn test_branch_newbranch() {
    let cliente = "cliente_branch".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(vec!["test_branch".to_string()], cliente.clone()).unwrap();
    let _ = write_file(
        (cliente.clone() + "/gitrconfig").to_string(),
        ("[user]\n\tname = test\n\temail = test@gmail.com").to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_branch/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_branch/blob2").to_string(),
        "Hello, im blob 2".to_string(),
    );
    commands_fn::add(vec!["blob1".to_string()], cliente.clone()).unwrap();
    commands_fn::add(vec!["blob2".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"commit 1\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();
    commands_fn::branch(vec!["branch1".to_string()], cliente.clone()).unwrap();
    let res = print_branches(cliente.clone()).unwrap();
    let correct_res = String::from("* \x1b[92mmaster\x1b[0m\nbranch1\n");
    assert_eq!(res, correct_res);
    fs::remove_dir_all(cliente.clone()).unwrap();
}

#[test]
#[serial]
fn test_branch_no_commit() {
    let cliente = "cliente_branch_no_commit".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(vec!["test_branch_no_commit".to_string()], cliente.clone()).unwrap();
    let error = commands_fn::branch(vec!["branch1".to_string()], cliente.clone()).unwrap_err();
    let res = print_branches(cliente.clone()).unwrap();
    let correct_res = String::from("");
    assert_eq!(res, correct_res);
    assert!(matches!(error, GitrError::NoCommitExisting(_)));
    fs::remove_dir_all(cliente.clone()).unwrap();
}

#[test]
#[serial]
fn test_branch_already_exists() {
    let cliente = "cliente_branch_already_exists".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(
        vec!["test_branch_already_exists".to_string()],
        cliente.clone(),
    )
    .unwrap();
    let _ = write_file(
        (cliente.clone() + "/gitrconfig").to_string(),
        ("[user]\n\tname = test\n\temail =test@gmail.com").to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_branch_already_exists/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_branch_already_exists/blob2").to_string(),
        "Hello, im blob 2".to_string(),
    );
    commands_fn::add(vec!["blob1".to_string()], cliente.clone()).unwrap();
    commands_fn::add(vec!["blob2".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"commit 1\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();
    commands_fn::branch(vec!["branch1".to_string()], cliente.clone()).unwrap();
    let error = commands_fn::branch(vec!["branch1".to_string()], cliente.clone()).unwrap_err();
    assert!(matches!(error, GitrError::BranchAlreadyExistsError(_)));
    fs::remove_dir_all(cliente.clone()).unwrap();
}

#[test]
#[serial]
fn test_branch_delete() {
    let cliente = "cliente_branch_delete".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(vec!["test_branch_delete".to_string()], cliente.clone()).unwrap();
    let _ = write_file(
        (cliente.clone() + "/gitrconfig").to_string(),
        ("[user]\n\tname = test\n\temail =test@gmail.com").to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_branch_delete/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_branch_delete/blob2").to_string(),
        "Hello, im blob 2".to_string(),
    );
    commands_fn::add(vec!["blob1".to_string()], cliente.clone()).unwrap();
    commands_fn::add(vec!["blob2".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"commit 1\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();
    commands_fn::branch(vec!["branch1".to_string()], cliente.clone()).unwrap();
    let res = print_branches(cliente.clone()).unwrap();
    let correct_res = String::from("* \x1b[92mmaster\x1b[0m\nbranch1\n");
    assert_eq!(res, correct_res);
    commands_fn::branch(
        vec!["-d".to_string(), "branch1".to_string()],
        cliente.clone(),
    )
    .unwrap();
    let res = print_branches(cliente.clone()).unwrap();
    let correct_res = String::from("* \x1b[92mmaster\x1b[0m\n");
    assert_eq!(res, correct_res);
    fs::remove_dir_all(cliente.clone()).unwrap();
}

#[test]
#[serial]
fn test_branch_delete_current() {
    let cliente = "cliente_branch_delete_current".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(
        vec!["test_branch_delete_current".to_string()],
        cliente.clone(),
    )
    .unwrap();
    let _ = write_file(
        (cliente.clone() + "/gitrconfig").to_string(),
        ("[user]\n\tname = test\n\temail =test@gmail.com").to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_branch_delete_current/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_branch_delete_current/blob2").to_string(),
        "Hello, im blob 2".to_string(),
    );
    commands_fn::add(vec!["blob1".to_string()], cliente.clone()).unwrap();
    commands_fn::add(vec!["blob2".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"commit 1\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();
    let error = commands_fn::branch(
        vec!["-d".to_string(), "master".to_string()],
        cliente.clone(),
    )
    .unwrap_err();
    assert!(matches!(error, GitrError::DeleteCurrentBranchError(_)));
    fs::remove_dir_all(cliente.clone()).unwrap();
}

#[test]
#[serial]
fn test_branch_move() {
    let cliente = "cliente_branch_move".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(vec!["test_branch_move".to_string()], cliente.clone()).unwrap();
    let _ = write_file(
        (cliente.clone() + "/gitrconfig").to_string(),
        ("[user]\n\tname = test\n\temail =test@gmail.com").to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_branch_move/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_branch_move/blob2").to_string(),
        "Hello, im blob 2".to_string(),
    );
    commands_fn::add(vec!["blob1".to_string()], cliente.clone()).unwrap();
    commands_fn::add(vec!["blob2".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"commit 1\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();
    commands_fn::branch(vec!["branch1".to_string()], cliente.clone()).unwrap();
    commands_fn::branch(
        vec![
            "-m".to_string(),
            "branch1".to_string(),
            "branch2".to_string(),
        ],
        cliente.clone(),
    )
    .unwrap();
    let res = print_branches(cliente.clone()).unwrap();
    let correct_res = String::from("branch2\n* \x1b[92mmaster\x1b[0m\n");
    fs::remove_dir_all(cliente.clone()).unwrap();
    assert_eq!(res, correct_res);
}

// /*********************
//   HASH-OBJECT TESTS
// *********************/
#[test]
#[serial]
fn test_hash_object() {
    let cliente = "cliente_hash_object".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(vec!["test_hash_object".to_string()], cliente.clone()).unwrap();
    let _ = write_file(
        (cliente.clone() + "/test_hash_object/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    );
    let correct_hash = "016a41a6a35d50d311286359f1a7611948a9c529";
    let res = get_object_hash(cliente.clone(), &mut ("blob1").to_string(), false).unwrap();
    fs::remove_dir_all(cliente.clone()).unwrap();
    assert_eq!(res, correct_hash);
}

// /*********************
//   CAT-FILE TESTS
// *********************/
#[test]
#[serial]
fn test_cat_file() {
    let cliente = "cliente_cat_file".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(vec!["test_cat_file".to_string()], cliente.clone()).unwrap();
    let _ = write_file(
        (cliente.clone() + "/test_cat_file/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_cat_file/blob2").to_string(),
        "Hello, im blob 2".to_string(),
    );
    commands_fn::add(vec!["blob1".to_string()], cliente.clone()).unwrap();
    let hash1 = Blob::new("Hello, im blob 1".to_string())
        .unwrap()
        .get_hash();
    let res = _cat_file(vec!["-p".to_string(), hash1.clone()], cliente.clone()).unwrap();
    let correct_res = String::from("Hello, im blob 1");
    assert_eq!(res, correct_res);
    let res = _cat_file(vec!["-t".to_string(), hash1], cliente.clone()).unwrap();
    let correct_res = String::from("blob");
    assert_eq!(res, correct_res);
    fs::remove_dir_all(cliente.clone()).unwrap();
}

// /*********************
//   MERGE TESTS
// *********************/
fn refresh_files() {
    delete_repo("cliente/test".to_string());
    let cliente = "cliente".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(vec!["test".to_string()], cliente.clone()).unwrap();
    file_manager::write_file(
        (cliente.clone() + "/gitrconfig").to_string(),
        "[user]\n\tname = cliente\n\temail = cliente@gmail.com".to_string(),
    )
    .unwrap();
}

fn delete_repo(repo_path: String) {
    let path = Path::new(&repo_path);
    println!("path: {:?}", path);
    if path.exists() {
        fs::remove_dir_all(path).unwrap();
    }
}

#[test]
#[serial]
fn merge_con_archivos_test_1_conflict_trivial() {
    refresh_files();
    let base = vec![
        "hola\n",
        "\n",
        "\n",
        "\n",
        "linea de base\n",
        "\n",
        "\n",
        "\n",
        "chau\n",
    ]
    .concat();

    let origin = vec![
        "hola\n",
        "\n",
        "\n",
        "\n",
        "linea de origin\n",
        "\n",
        "\n",
        "linea agregada de origin\n",
        "chau\n",
    ]
    .concat();

    let branch = vec![
        "hola\n",
        "\n",
        "linea agregada de branch\n",
        "\n",
        "linea de branch\n",
        "\n",
        "\n",
        "\n",
        "chau\n",
    ]
    .concat();

    let cliente = "cliente".to_string();

    file_manager::write_file("cliente/test/archivo1.txt".to_string(), base).unwrap();
    commands_fn::add(vec![".".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"base\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();

    commands_fn::checkout(
        vec!["-b".to_string(), "branch".to_string()],
        cliente.clone(),
    )
    .unwrap();
    file_manager::write_file("cliente/test/archivo1.txt".to_string(), branch).unwrap();
    commands_fn::add(vec![".".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"branch\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();

    commands_fn::checkout(vec!["master".to_string()], cliente.clone()).unwrap();
    file_manager::write_file("cliente/test/archivo1.txt".to_string(), origin).unwrap();
    commands_fn::add(vec![".".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"origin\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();

    commands_fn::merge(vec!["branch".to_string()], cliente.clone()).unwrap();

    let archivo_esperado = vec![
        "hola\n",
        "\n",
        "linea agregada de branch\n",
        "\n",
        "<<<<<<< HEAD\n",
        "linea de origin\n",
        "=======\n",
        "linea de branch\n",
        ">>>>>>> BRANCH\n",
        "\n",
        "\n",
        "linea agregada de origin\n",
        "chau\n",
    ]
    .concat();

    let archivo_mergeado =
        file_manager::read_file("cliente/test/archivo1.txt".to_string()).unwrap();

    assert_eq!(archivo_mergeado, archivo_esperado);
    delete_repo("cliente/test".to_string());
}

#[test]
#[serial]
fn merge_con_archivos_test_3_conflict_multilinea() {
    refresh_files();
    let base = vec![
        "hola\n",
        "\n",
        "\n",
        "\n",
        "linea de base 1\n",
        "linea de base 2\n",
        "linea de base 3\n",
        "linea de base 4\n",
        "\n",
        "\n",
        "\n",
        "chau\n",
    ]
    .concat();

    let origin = vec![
        "hola\n",
        "\n",
        "\n",
        "\n",
        "linea de origin 1\n",
        "linea de origin 2\n",
        "linea de origin 3\n",
        "\n",
        "\n",
        "\n",
        "chau\n",
    ]
    .concat();

    let branch = vec![
        "hola\n",
        "\n",
        "\n",
        "\n",
        "linea de branch 1\n",
        "linea de branch 2\n",
        "linea de branch 3\n",
        "linea de branch 4\n",
        "linea de branch 5\n",
        "\n",
        "\n",
        "\n",
        "chau\n",
    ]
    .concat();

    let cliente = "cliente".to_string();

    file_manager::write_file("cliente/test/archivo1.txt".to_string(), base).unwrap();
    commands_fn::add(vec![".".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"base\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();

    commands_fn::checkout(
        vec!["-b".to_string(), "branch".to_string()],
        cliente.clone(),
    )
    .unwrap();
    file_manager::write_file("cliente/test/archivo1.txt".to_string(), branch).unwrap();
    commands_fn::add(vec![".".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"branch\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();

    commands_fn::checkout(vec!["master".to_string()], cliente.clone()).unwrap();
    file_manager::write_file("cliente/test/archivo1.txt".to_string(), origin).unwrap();
    commands_fn::add(vec![".".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"origin\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();

    commands_fn::merge(vec!["branch".to_string()], cliente.clone()).unwrap();

    let archivo_esperado = vec![
        "hola\n",
        "\n",
        "\n",
        "\n",
        "<<<<<<< HEAD\n",
        "linea de origin 1\n",
        "linea de origin 2\n",
        "linea de origin 3\n",
        "=======\n",
        "linea de branch 1\n",
        "linea de branch 2\n",
        "linea de branch 3\n",
        "linea de branch 4\n",
        "linea de branch 5\n",
        ">>>>>>> BRANCH\n",
        "\n",
        "\n",
        "chau\n",
    ]
    .concat();

    let archivo_mergeado =
        file_manager::read_file("cliente/test/archivo1.txt".to_string()).unwrap();

    assert_eq!(archivo_mergeado, archivo_esperado);
    delete_repo("cliente/test".to_string());
}

#[test]
#[serial]
fn merge_con_archivos_test_4_multiples_conflicts_multilinea() {
    refresh_files();
    let base = vec![
        "hola\n",
        "\n",
        "\n",
        "\n",
        "linea de base 1\n",
        "linea de base 2\n",
        "linea de base 3\n",
        "linea igual para todos\n",
        "linea de base 5\n",
        "linea de base 6\n",
        "linea de base 7\n",
        "\n",
        "chau\n",
    ]
    .concat();

    let origin = vec![
        "hola\n",
        "\n",
        "\n",
        "\n",
        "linea de origin 1\n",
        "linea de origin 2\n",
        "linea de origin 3\n",
        "linea igual para todos\n",
        "linea de origin 4\n",
        "linea de origin 5\n",
        "linea de origin 6\n",
        "\n",
        "chau\n",
    ]
    .concat();

    let branch = vec![
        "hola\n",
        "\n",
        "\n",
        "\n",
        "linea de branch 1\n",
        "linea de branch 2\n",
        "linea de branch 3\n",
        "linea igual para todos\n",
        "linea de branch 4\n",
        "\n",
        "chau\n",
    ]
    .concat();

    let cliente = "cliente".to_string();

    file_manager::write_file("cliente/test/archivo1.txt".to_string(), base).unwrap();
    commands_fn::add(vec![".".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"base\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();

    commands_fn::checkout(
        vec!["-b".to_string(), "branch".to_string()],
        cliente.clone(),
    )
    .unwrap();
    file_manager::write_file("cliente/test/archivo1.txt".to_string(), branch).unwrap();
    commands_fn::add(vec![".".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"branch\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();

    commands_fn::checkout(vec!["master".to_string()], cliente.clone()).unwrap();
    file_manager::write_file("cliente/test/archivo1.txt".to_string(), origin).unwrap();
    commands_fn::add(vec![".".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"origin\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();

    commands_fn::merge(vec!["branch".to_string()], cliente.clone()).unwrap();

    let archivo_esperado = vec![
        "hola\n",
        "\n",
        "\n",
        "\n",
        "<<<<<<< HEAD\n",
        "linea de origin 1\n",
        "linea de origin 2\n",
        "linea de origin 3\n",
        "=======\n",
        "linea de branch 1\n",
        "linea de branch 2\n",
        "linea de branch 3\n",
        ">>>>>>> BRANCH\n",
        "linea igual para todos\n",
        "<<<<<<< HEAD\n",
        "linea de origin 4\n",
        "linea de origin 5\n",
        "linea de origin 6\n",
        "=======\n",
        "linea de branch 4\n",
        ">>>>>>> BRANCH\n",
        "\n",
        "chau\n",
    ]
    .concat();

    let archivo_mergeado =
        file_manager::read_file("cliente/test/archivo1.txt".to_string()).unwrap();

    assert_eq!(archivo_mergeado, archivo_esperado);
    delete_repo("cliente/test".to_string());
}

#[test]
#[serial]
fn merge_con_archivos_test_5_ejemplo_de_codigo() {
    refresh_files();
    let base = vec![
        "fn main() {\n",
        "    let a = 1;\n",
        "    let b = 2;\n",
        "\n",
        "    if a == b {\n",
        "        println!(\"iguales\");\n",
        "    } else {\n",
        "        println!(\"distintos\");\n",
        "    }\n",
        "}\n",
    ]
    .concat();

    let origin = vec![
        "fn main() {\n",
        "    let a = 1;\n",
        "    let origin_variable = 2;\n",
        "\n",
        "    if a == b {\n",
        "        println!(\"iguales\");\n",
        "        let res = origin_function();\n",
        "    } else {\n",
        "        println!(\"distintos\");\n",
        "    }\n",
        "}\n",
    ]
    .concat();

    let branch = vec![
        "fn main() {\n",
        "    let a = 1;\n",
        "    let branch_variable = 2;\n",
        "\n",
        "    if a == b {\n",
        "        println!(\"iguales\");\n",
        "        let res = branch_function();\n",
        "        println!(\"res: {}\", res);\n",
        "    } else {\n",
        "        println!(\"distintos\");\n",
        "    }\n",
        "}\n",
    ]
    .concat();

    let cliente = "cliente".to_string();

    file_manager::write_file("cliente/test/archivo1.txt".to_string(), base).unwrap();
    commands_fn::add(vec![".".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"base\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();

    commands_fn::checkout(
        vec!["-b".to_string(), "branch".to_string()],
        cliente.clone(),
    )
    .unwrap();
    file_manager::write_file("cliente/test/archivo1.txt".to_string(), branch).unwrap();
    commands_fn::add(vec![".".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"branch\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();

    commands_fn::checkout(vec!["master".to_string()], cliente.clone()).unwrap();
    file_manager::write_file("cliente/test/archivo1.txt".to_string(), origin).unwrap();
    commands_fn::add(vec![".".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"origin\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();

    commands_fn::merge(vec!["branch".to_string()], cliente.clone()).unwrap();

    let archivo_esperado = vec![
        "fn main() {\n",
        "    let a = 1;\n",
        "<<<<<<< HEAD\n",
        "    let origin_variable = 2;\n",
        "=======\n",
        "    let branch_variable = 2;\n",
        ">>>>>>> BRANCH\n",
        "\n",
        "    if a == b {\n",
        "        println!(\"iguales\");\n",
        "<<<<<<< HEAD\n",
        "        let res = origin_function();\n",
        "=======\n",
        "        let res = branch_function();\n",
        "        println!(\"res: {}\", res);\n",
        ">>>>>>> BRANCH\n",
        "        let res = origin_function();\n",
        "    } else {\n",
        "        println!(\"distintos\");\n",
        "    }\n",
        "}\n",
    ]
    .concat();

    let archivo_mergeado =
        file_manager::read_file("cliente/test/archivo1.txt".to_string()).unwrap();

    assert_eq!(archivo_mergeado, archivo_esperado);
    delete_repo("cliente/test".to_string());
}

// /*********************
//   COMMIT TESTS
// *********************/
#[test]
#[serial]
fn test_commit() {
    let cliente = "cliente_commit".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(vec!["test_commit".to_string()], cliente.clone()).unwrap();
    let _ = write_file(
        (cliente.clone() + "/gitrconfig").to_string(),
        "[user]\n\tname = test\n\temail =test@gmail.com".to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_commit/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_commit/blob2").to_string(),
        "Hello, im blob 2".to_string(),
    );
    commands_fn::add(vec![".".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"commit 1\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();
    let res =
        file_manager::read_file(cliente.clone() + "/test_commit/gitr/refs/heads/master").unwrap();
    let current_commit = file_manager::get_current_commit(cliente.clone()).unwrap();
    assert_eq!(res, current_commit);
    fs::remove_dir_all(cliente.clone()).unwrap();
}

// /*********************
//   CHECKOUT TESTS
// *********************/
#[test]
#[serial]
fn test_checkout() {
    let cliente = "cliente_checkout".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(vec!["test_checkout".to_string()], cliente.clone()).unwrap();
    let _ = write_file(
        (cliente.clone() + "/gitrconfig").to_string(),
        "[user]\n\tname = test\n\temail =test@gmail.com".to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_checkout/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    );
    commands_fn::add(vec![".".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"commit 1\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();
    commands_fn::branch(vec!["branch1".to_string()], cliente.clone()).unwrap();
    commands_fn::checkout(vec!["branch1".to_string()], cliente.clone()).unwrap();
    let _ = write_file(
        (cliente.clone() + "/test_checkout/blob1").to_string(),
        "Hello, im blob 1 in branch1".to_string(),
    );
    commands_fn::add(vec![".".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"commit 2\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();
    commands_fn::checkout(vec!["master".to_string()], cliente.clone()).unwrap();
    let res = file_manager::read_file(cliente.clone() + "/test_checkout/blob1").unwrap();
    let correct_res = String::from("Hello, im blob 1");
    assert_eq!(res, correct_res);
    fs::remove_dir_all(cliente.clone()).unwrap();
}

// /*********************
//   STATUS TESTS
// *********************/
#[test]
#[serial]
fn test_status_new_file_untracked() {
    let cliente = "cliente_status_new_file_added".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(
        vec!["test_status_new_file_added".to_string()],
        cliente.clone(),
    )
    .unwrap();
    let _ = write_file(
        (cliente.clone() + "/gitrconfig").to_string(),
        "[user]\n\tname = test\n\temail =test@gmail.com".to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_status_new_file_added/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    );
    let (not_staged, untracked_files, hayindex) =
        get_untracked_notstaged_files(cliente.clone()).unwrap();

    assert!(untracked_files.contains(&(cliente.clone() + "/test_status_new_file_added/blob1")));
    assert!(!not_staged.contains(&"blob1".to_string()));
    assert!(!hayindex);
    fs::remove_dir_all(cliente.clone()).unwrap();
}

#[test]
#[serial]
fn test_status_new_file_added() {
    let cliente = "cliente_status_new_file_added".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(
        vec!["test_status_new_file_added".to_string()],
        cliente.clone(),
    )
    .unwrap();
    let _ = write_file(
        (cliente.clone() + "/gitrconfig").to_string(),
        ("[user]\n\tname = test\n\temail =test@gmail.com").to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_status_new_file_added/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    );
    commands_fn::add(vec!["blob1".to_string()], cliente.clone()).unwrap();
    let (not_staged, _, _) = get_untracked_notstaged_files(cliente.clone()).unwrap();
    let (new_files, modified_files) =
        get_tobe_commited_files(&not_staged, cliente.clone()).unwrap();
    assert!(new_files.contains(&(cliente.clone() + "/test_status_new_file_added/blob1")));
    assert!(!modified_files.contains(&"blob1".to_string()));
    fs::remove_dir_all(cliente.clone()).unwrap();
}

#[test]
#[serial]
fn test_status_new_file_modified() {
    let cliente = "cliente_status_new_file_modified".to_string();
    fs::create_dir_all(Path::new(&cliente)).unwrap();
    commands_fn::init(
        vec!["test_status_new_file_modified".to_string()],
        cliente.clone(),
    )
    .unwrap();
    let _ = write_file(
        (cliente.clone() + "/gitrconfig").to_string(),
        ("[user]\n\tname = test\n\temail =g@gmail.com").to_string(),
    );
    let _ = write_file(
        (cliente.clone() + "/test_status_new_file_modified/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    );
    commands_fn::add(vec!["blob1".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"commit 1\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();
    let _ = write_file(
        (cliente.clone() + "/test_status_new_file_modified/blob1").to_string(),
        "Hello, im blob 1 modified".to_string(),
    );
    commands_fn::add(vec!["blob1".to_string()], cliente.clone()).unwrap();
    let (not_staged, _, _) = get_untracked_notstaged_files(cliente.clone()).unwrap();
    let (new_files, modified_files) =
        get_tobe_commited_files(&not_staged, cliente.clone()).unwrap();
    assert!(!new_files.contains(&(cliente.clone() + "/test_status_new_file_modified/blob1")));
    assert!(modified_files.contains(&(cliente.clone() + "/test_status_new_file_modified/blob1")));
    fs::remove_dir_all(cliente.clone()).unwrap();
}

// /*********************
//   LS-TREE TESTS
// *********************/
#[test]
#[serial]
fn test_ls_tree_sin_flags_se_comporta_como_cat_file() {
    refresh_files();
    let cliente = "cliente".to_string();

    write_file(
        (cliente.clone() + "/test/blob1").to_string(),
        "Hello, im blob 1".to_string(),
    )
    .unwrap();
    commands_fn::add(vec!["blob1".to_string()], cliente.clone()).unwrap();
    commands_fn::commit(
        vec!["-m".to_string(), "\"commit 1\"".to_string()],
        "None".to_string(),
        cliente.clone(),
    )
    .unwrap();

    let current_commit = file_manager::get_current_commit(cliente.clone()).unwrap();

    let commit =
        file_manager::read_object(&current_commit, cliente.clone() + "/test/gitr/", false).unwrap();

    let _tree_hash = commit.split(" ").collect::<Vec<&str>>()[2].to_string();
    let tree_hash = _tree_hash.split("\n").collect::<Vec<&str>>()[0].to_string();
    let res =
        command_utils::_ls_tree(vec![tree_hash.clone()], "".to_string(), cliente.clone()).unwrap();

    let cat_file = _cat_file(vec!["-p".to_string(), tree_hash], cliente.clone()).unwrap();

    assert_eq!(res, cat_file);
}
