use crate::commands::command_utils::{flate2compress, sha1hashing};
use crate::file_manager;
use crate::gitr_errors::GitrError;
use crate::objects::tree::Tree;
use std::fmt::Write;

#[derive(Debug)]

pub enum TreeEntry {
    Blob(Blob),
    Tree(Tree),
}

#[derive(Debug)]
pub struct Blob {
    compressed_data: Vec<u8>,
    hash: String,
}

impl Blob {
    pub fn new(raw_data: String) -> Result<Self, GitrError> {
        let format_data = format!("blob {}\0{}", raw_data.as_bytes().len(), raw_data);
        let compressed_data = flate2compress(format_data.clone())?;
        let hashed_file = sha1hashing(format_data);
        let hashed_file_str = hashed_file
            .iter()
            .fold(String::new(),|mut output,b| {
                let _ =write!(output,"{b:02x}");
                output
            });
        Ok(Blob {
            compressed_data,
            hash: hashed_file_str,
        })
    }
    pub fn save(&self, cliente: String) -> Result<(), GitrError> {
        file_manager::write_object(self.compressed_data.clone(), self.get_hash(), cliente)?;
        Ok(())
    }

    pub fn get_hash(&self) -> String {
        self.hash.clone()
    }

    pub fn get_data(&self) -> Vec<u8> {
        self.compressed_data.clone()
    }
}
