use super::commands_fn;
use crate::{gitr_errors::GitrError, logger};

pub fn parse_input(input: String) -> Vec<String> {
    return input.split_whitespace().map(|s| s.to_string()).collect();
}

/// ["command", "flag1", "flag2", ...]
pub fn command_handler(
    argv: Vec<String>,
    hubo_conflict: bool,
    branch_hash: String,
    client: String,
) -> Result<(bool, String), GitrError> {
    if argv.is_empty() {
        return Ok((false, "".to_string()));
    }

    let command = argv[0].clone();

    let flags = argv[1..].to_vec();

    let message = format!("calling {} with flags: {:?}", command, flags);
    match logger::log_action(message.clone()) {
        Ok(_) => (),
        Err(e) => println!("Error: {}", e),
    };

    match command.as_str() {
        "hash-object" | "h" => commands_fn::hash_object(flags, client)?, //"h" para testear mas rapido mientras la implementamos
        "cat-file" | "c" => commands_fn::cat_file(flags, client)?,
        "init" => commands_fn::init(flags, client)?,
        "status" => commands_fn::status(flags, client)?,
        "add" => {
            commands_fn::add(flags, client)?;
            return Ok((hubo_conflict, branch_hash));
        }
        "rm" => commands_fn::rm(flags, client)?,
        "commit" => {
            if hubo_conflict {
                commands_fn::commit(flags, branch_hash.clone(), client)?;
                return Ok((false, "".to_string()));
            } else {
                commands_fn::commit(flags, "None".to_string(), client)?;
            }
        }
        "checkout" => commands_fn::checkout(flags, client)?,
        "log" => commands_fn::log(flags, client)?,
        "clone" => commands_fn::clone(flags, client)?,
        "fetch" => commands_fn::fetch(flags, client)?,
        "merge" => {
            let (hubo_conflict_res, branch_hash_res, _) = commands_fn::merge(flags, client)?;
            if hubo_conflict_res {
                println!(
                    "\x1b[33mHubo un conflicto, por favor resuelvalo antes de continuar\x1b[0m"
                );
            }
            return Ok((true, branch_hash_res));
        }
        "remote" => commands_fn::remote(flags, client)?,
        "pull" => commands_fn::pull(flags, client)?,
        "push" => commands_fn::push(flags, client)?,
        "branch" => commands_fn::branch(flags, client)?,
        "ls-files" => commands_fn::ls_files(flags, client)?,
        "show-ref" => commands_fn::show_ref(flags, client)?,
        "tag" => commands_fn::tag(flags, client)?,
        "ls-tree" => commands_fn::ls_tree(flags, client)?,
        "rebase" => commands_fn::rebase(flags, client)?,
        "check-ignore" => commands_fn::check_ignore(flags, client)?,
        "q" => return Ok((false, "".to_string())),
        "l" => logger::log(flags)?,
        "list-repos" | "lr" => commands_fn::list_repos(client),
        "go-to-repo" | "gtr" => commands_fn::go_to_repo(flags, client)?,
        "cur-repo" | "cr" => commands_fn::print_current_repo(client)?,
        "echo" => commands_fn::echo(flags, client)?,
        _ => {
            let message = format!("invalid command: {}", command);
            return Err(GitrError::InvalidArgumentError(
                message,
                "usage: gitr <command> [<args>]".to_string(),
            ));
        }
    }

    Ok((false, "".to_string()))
}
