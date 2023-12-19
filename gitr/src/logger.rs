use ::chrono;

use crate::{file_manager, gitr_errors::GitrError};

#[derive(Debug)]
enum EntryType {
    Error,
    Action,
    FileOperation,
}

struct Logger {
    entry_type: EntryType,
    timestamp: String,
    message: String,
}

impl Logger {
    pub fn new(entry_type: EntryType, message: String) -> Self {
        let now = chrono::Local::now();
        let timestamp = now.format("%d-%m-%Y %H:%M:%S").to_string();
        Logger {
            entry_type,
            timestamp,
            message,
        }
    }

    pub fn save(&self) -> Result<(), GitrError> {
        let entry = format!(
            "{{\"type\": \"{:?}\",\"timestamp\": \"{}\",\"message\": \"{}\"}}",
            self.entry_type, self.timestamp, self.message
        );
        match file_manager::append_to_file("src/log.json".to_string(), entry) {
            Ok(_) => Ok(()),
            Err(_) => Err(GitrError::LogError),
        }
    }
}

pub fn log_error(message: String) -> Result<(), GitrError> {
    let logger = Logger::new(EntryType::Error, message);
    logger.save()?;
    Ok(())
}

pub fn log_action(message: String) -> Result<(), GitrError> {
    let logger = Logger::new(EntryType::Action, message);

    logger.save()?;
    Ok(())
}

pub fn log_file_operation(message: String) -> Result<(), GitrError> {
    let logger = Logger::new(EntryType::FileOperation, message);
    logger.save()?;
    Ok(())
}

pub fn log(flags: Vec<String>) -> Result<(), GitrError> {
    let n = match flags[0].parse::<usize>() {
        Ok(n) => n,
        Err(_) => {
            return Err(GitrError::InvalidArgumentError(
                flags[0].clone(),
                "log <n>".to_string(),
            ))
        }
    };

    let log = file_manager::read_file("src/log.json".to_string())?;
    for line in log.lines().rev().take(n) {
        let msg = line.split("message\": ").collect::<Vec<&str>>()[1];
        if line.contains("Error") {
            println!("\x1b[31m{}\x1b[0m", msg);
        } else if line.contains("Action") {
            println!("\x1b[34m{}\x1b[0m", msg);
        } else if line.contains("FileOperation") {
            println!("\x1b[93m{}\x1b[0m", msg);
        } else {
            println!("{}", msg);
        }
    }

    Ok(())
}
