use std::collections::HashSet;
use std::fmt::Write;

use super::blob::TreeEntry;
use crate::gitr_errors::GitrError;
use crate::{commands, file_manager};

#[derive(Debug)]
pub struct Tree {
    data: Vec<u8>,
    hash: String,
}

pub fn get_formated_hash(hash: String, path: &str) -> Result<Vec<u8>, GitrError> {
    let mut formated_hash: Vec<u8> = Vec::new();
    for i in (0..40).step_by(2) {
        let first_char = hash.as_bytes()[i] as char;
        let second_char = hash.as_bytes()[i + 1] as char;
        let byte_as_str = format!("{}{}", first_char, second_char);
        let byte = match u8::from_str_radix(&byte_as_str, 16) {
            Ok(byte) => byte,
            Err(_) => return Err(GitrError::FileReadError(path.to_string())),
        };
        formated_hash.push(byte);
    }
    Ok(formated_hash)
}

impl Tree {
    pub fn new(mut entries: Vec<(String, TreeEntry)>) -> Result<Self, GitrError> {
        entries.sort_by(|a, b| a.0.cmp(&b.0));

        let mut objs_entries = Vec::new();
        let mut entries_size: usize = 0;
        for (path, entry) in &entries {
            match entry {
                TreeEntry::Blob(blob) => {
                    let hash = blob.get_hash();
                    let formated_hash = get_formated_hash(hash, path)?;

                    let _path_no_repo = path.split_once('/').unwrap().1;
                    let file_name = path.split('/').last().unwrap();
                    let obj_entry =
                        [b"100644 ", file_name.as_bytes(), b"\0", &formated_hash].concat();

                    entries_size += obj_entry.len();
                    objs_entries.push(obj_entry);
                }
                TreeEntry::Tree(tree) => {
                    let hash = tree.get_hash();
                    let formated_hash = get_formated_hash(hash, path)?;

                    let obj_entry = [b"40000 ", path.as_bytes(), b"\0", &formated_hash].concat();

                    entries_size += obj_entry.len();
                    objs_entries.push(obj_entry);
                }
            }
        }

        let data = [
            b"tree ",
            entries_size.to_string().as_bytes(),
            b"\0",
            &objs_entries.concat(),
        ]
        .concat();

        let compressed_file2 = commands::command_utils::flate2compress2(data.clone())?;
        let hashed_file2 = commands::command_utils::sha1hashing2(data.clone());

        let hashed_file_str = hashed_file2
            .iter()
            .fold(String::new(),|mut output,b| {
                let _ =write!(output,"{b:02x}");
                output
            });

        let mut format_data = String::new();
        let init = format!("tree {}\0", entries.len());
        format_data.push_str(&init);
        Ok(Tree {
            /*entries, */ data: compressed_file2,
            hash: hashed_file_str,
        })
    }

    pub fn new_from_packfile(raw_data: &[u8]) -> Result<Self, GitrError> {
        let header_len = raw_data.len();
        let tree_raw_file = [b"tree ", header_len.to_string().as_bytes(), b"\0", raw_data].concat();

        let compressed_data = commands::command_utils::flate2compress2(tree_raw_file.clone())?;
        let hash = commands::command_utils::sha1hashing2(tree_raw_file.clone());
        let tree_hash = hash
            .iter()
            .fold(String::new(),|mut output,b| {
                let _ =write!(output,"{b:02x}");
                output
            });

        let tree = Tree {
            /*entries: vec![],*/ data: compressed_data,
            hash: tree_hash,
        };
        Ok(tree)
    }

    pub fn save(&self, cliente: String) -> Result<(), GitrError> {
        file_manager::write_object(self.data.clone(), self.hash.clone(), cliente)?;
        Ok(())
    }

    pub fn get_hash(&self) -> String {
        self.hash.clone()
    }

    pub fn get_data(&self) -> Vec<u8> {
        self.data.clone()
    }

    pub fn get_objects_id_from_string(data: String) -> Result<Vec<String>, GitrError> {
        if data.split_at(4).0 != "tree" {
            return Err(GitrError::InvalidTreeError);
        }

        let mut objects_id = Vec::new();

        let raw_data = match data.split_once('\0') {
            Some((_, raw_data)) => raw_data,
            None => {
                println!("Error: invalid object type");
                return Err(GitrError::InvalidTreeError);
            }
        };
        for entry in raw_data.split('\n') {
            let _new_path_hash = entry.split(' ').collect::<Vec<&str>>()[1];
            let hash = _new_path_hash.split('\0').collect::<Vec<&str>>()[1];
            objects_id.push(hash.to_string());
        }
        Ok(objects_id)
    }

    pub fn get_all_tree_objects(
        tree_id: String,
        r_path: String,
        object_ids: &mut HashSet<String>,
    ) -> Result<(), GitrError> {
        if let Ok(tree_str) = file_manager::read_object(&tree_id, r_path.clone(), false) {
            let tree_objects = match Tree::get_objects_id_from_string(tree_str) {
                Ok(ids) => ids,
                _ => return Err(GitrError::InvalidTreeError),
            };
            for obj_id in tree_objects {
                object_ids.insert(obj_id.clone());
                let _ = Self::get_all_tree_objects(obj_id.clone(), r_path.clone(), object_ids);
            }

            return Ok(());
        }
        Err(GitrError::InvalidTreeError)
    }
}
