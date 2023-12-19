use chrono::Utc;
use std::fmt::Write;

use crate::{
    commands::command_utils::{
        flate2compress, get_current_username, get_user_mail_from_config, sha1hashing,
    },
    gitr_errors::GitrError,
};

#[derive(Debug)]

pub struct Tag {
    data: Vec<u8>,
    hash: String,
    commit_hash: String,
}

impl Tag {
    pub fn new(
        tag_name: String,
        tag_message: String,
        commit_hash: String,
        cliente: String,
    ) -> Result<Self, GitrError> {
        let mut format_data = String::new();
        format_data.push_str(&format!("object {}\n", commit_hash));
        format_data.push_str("type commit\n");
        format_data.push_str(&format!("tag {}\n", tag_name));
        format_data.push_str(&format!(
            "tagger {} <{}> {} -0300\n",
            get_current_username(cliente.clone()),
            get_user_mail_from_config(cliente.clone())?,
            Utc::now().timestamp()
        ));
        format_data.push_str('\n'.to_string().as_str());
        format_data.push_str(&format!("{}\n", tag_message));
        let size = format_data.as_bytes().len();
        let format_data_entera = format!("tag {}\0{}", size, format_data);
        let compressed_file = flate2compress(format_data_entera.clone())?;
        let hashed_file = sha1hashing(format_data_entera.clone());
        let hashed_file_str = hashed_file
            .iter()
            .fold(String::new(),|mut output,b| {
                let _ =write!(output,"{b:02x}");
                output
            });
        Ok(Tag {
            data: compressed_file,
            hash: hashed_file_str,
            commit_hash, /*tag_name: tag_name, tag_message: tag_message,commit_hash: commit_hash */
        })
    }

    pub fn new_tag_from_data(data: String) -> Result<Tag, GitrError> {
        let tag_elems = data.split('\0').collect::<Vec<&str>>();
        if tag_elems.len() != 2 || !tag_elems[0].contains("tag") {
            return Err(GitrError::InvalidTagError);
        }
        let tag_string = tag_elems[1].to_string();
        Self::new_tag_from_string(tag_string)
    }

    pub fn new_tag_from_string(data: String) -> Result<Tag, GitrError> {
        let (mut name, mut message, mut commit, mut tagger) =
            (String::new(), String::new(), String::new(), String::new());
        for line in data.lines() {
            let elems = line.split_once(' ').unwrap_or((line, ""));
            match elems.0 {
                "object" => commit = elems.1.to_string(),
                "tag" => name = elems.1.to_string(),
                "tagger" => tagger = elems.1.to_string(),
                _ => message = line.to_string(),
            }
        }
        let tag = Tag::new_from_packfile(name, "\n".to_string() + &message, commit, tagger)?;
        Ok(tag)
    }

    pub fn new_from_packfile(
        tag_name: String,
        tag_message: String,
        commit: String,
        tagger: String,
    ) -> Result<Tag, GitrError> {
        let mut format_data = String::new();
        format_data.push_str(&format!("object {}\n", commit));
        format_data.push_str("type commit\n");
        format_data.push_str(&format!("tag {}\n", tag_name));
        format_data.push_str(&format!("tagger {}", tagger));
        format_data.push('\n');
        format_data.push_str(&format!("{}\n", tag_message));
        let size = format_data.as_bytes().len();
        let format_data_entera = format!("tag {}\0{}", size, format_data);
        let compressed_file = flate2compress(format_data_entera.clone())?;
        let hashed_file = sha1hashing(format_data_entera.clone());
        let hashed_file_str = hashed_file
            .iter()
            .fold(String::new(),|mut output,b| {
                let _ =write!(output,"{b:02x}");
                output
            });
        Ok(Tag {
            data: compressed_file,
            hash: hashed_file_str,
            commit_hash: commit,
        })
    }

    pub fn save(&self, cliente: String) -> Result<(), GitrError> {
        crate::file_manager::write_object(self.data.clone(), self.hash.clone(), cliente)?;
        Ok(())
    }

    pub fn get_commit_hash(&self) -> String {
        self.commit_hash.clone()
    }

    pub fn get_hash(&self) -> String {
        self.hash.clone()
    }
    pub fn get_data(&self) -> Vec<u8> {
        self.data.clone()
    }
}
