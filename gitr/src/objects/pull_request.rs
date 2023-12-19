
use serde::{Serialize, Deserialize};

use crate::{gitr_errors::GitrError, file_manager};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PullRequest {
    pub id: u8,
    pub title: String,
    pub description: String,
    pub head: String, 
    pub base: String, 
    pub status: String,
}

impl PullRequest {
    pub fn new(
        id: u8,
        title: String,
        description: String,
        head: String,
        base: String,
        //commits: Vec<String>,
    ) -> Self {

        PullRequest {
            id,
            title,
            description,
            head,
            base,
            //commits,
            status: String::from("open"),
        }
    }

    pub fn to_string(&self) -> Result<String, GitrError> {
            
        match serde_json::to_string(&self) {
            Ok(json) => Ok(json),
            Err(_) => Err(GitrError::PullRequestWriteError),
        }
    }

    pub fn from_string(content: String) -> Result<Self, GitrError> {
        match serde_json::from_str(&content) {
            Ok(pr) => Ok(pr),
            Err(_) => Err(GitrError::PullRequestReadError),
        }    
    }

    pub fn get_branch_name(&self) -> String {
        self.head.clone()
    }

    pub fn get_base_name(&self) -> String {
        self.base.clone()
    }

    pub fn get_status(&self) -> &String {
        &self.status
    }

    pub fn close(&mut self, path: String) -> Result<(), GitrError> {
        self.status = String::from("closed");
        let data = self.to_string()?;
        file_manager::write_file(path, data)?;
        Ok(())
    }

}