use gitr::{commands, file_manager, gitr_errors::GitrError, logger};

use std::{
    fs,
    io::{self, Write},
};
extern crate flate2;

use gitr::gui::gui_from_glade::*;

fn get_input() -> Result<String, GitrError> {
    print!("\x1b[34mgitr: $ \x1b[0m");
    match io::stdout().flush() {
        Ok(_) => (),
        Err(e) => {
            return Err(GitrError::InvalidArgumentError(
                e.to_string(),
                "Usage: TODO".to_string(),
            ))
        }
    }
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => (),
        Err(e) => {
            return Err(GitrError::InvalidArgumentError(
                e.to_string(),
                "Usage: TODO".to_string(),
            ))
        }
    }
    Ok(input)
}

fn email_valido(email_recibido: String) -> bool {
    let email_parts: Vec<&str> = email_recibido.split('@').collect::<Vec<&str>>();

    if email_parts.len() != 2 {
        return false;
    }
    let domain = email_parts[1];
    if !domain.contains('.') {
        return false;
    }
    true
}

fn setup_config_file(client_path: String) {
    let mut email_recibido = String::new();

    while !email_valido(email_recibido.clone()) {
        println!("Ingrese su email: ");
        email_recibido = match get_input() {
            Ok(email) => email,
            Err(_) => "user@mail.com".to_string(),
        };
    }
    println!("El email es valido, ya puede comenzar a usar Gitr\n");
    let name = client_path.clone();
    let config_file_data = format!("[user]\n\temail = {}\n\tname = {}\n", email_recibido, name);
    file_manager::write_file(client_path + "/gitrconfig", config_file_data).unwrap();
}

pub fn existe_config(client_path: String) -> bool {
    fs::metadata(client_path + "/gitrconfig").is_ok()
}

fn print_bienvenida() {
    println!("\t╔══════════════════════════════════════════════╗");
    println!("\t║ \x1b[34mBienvenido a la version command-line de Gitr\x1b[0m ║");
    println!("\t║ \x1b[34mIntroduzca los comandos que desea realizar\x1b[0m   ║");
    println!("\t║ \x1b[34m(introduzca q para salir del programa)\x1b[0m       ║");
    println!("\t╚══════════════════════════════════════════════╝");
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        println!("Usage: cargo run --bin client <client_name>");
        return;
    }
    let cliente = args[1].clone();
    let cliente_clon = cliente.clone();

    let child = std::thread::spawn(move || {
        initialize_gui(cliente_clon.clone());
    });
    print_bienvenida();

    let _ = file_manager::create_directory(&cliente);
    while !existe_config(cliente.clone()) {
        setup_config_file(cliente.clone());
    }
    let mut hubo_conflict = false;
    let mut branch_hash = "".to_string();

    loop {
        let input = match get_input() {
            Ok(input) => input,
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
        };

        if input == "q\n" {
            return;
        }
        let argv: Vec<String> = commands::handler::parse_input(input);

        // argv = ["command", "flag1", "flag2", ...]
        match commands::handler::command_handler(
            argv,
            hubo_conflict,
            branch_hash.clone(),
            cliente.clone(),
        ) {
            Ok((hubo_conflict_res, branch_hash_res)) => {
                hubo_conflict = hubo_conflict_res;
                branch_hash = branch_hash_res;
            }
            Err(e) => {
                println!("{}", e);
                match logger::log_error(e.to_string()) {
                    Ok(_) => (),
                    Err(e) => println!("Logger Error: {}", e),
                };
            }
        };
    }
    match child.join() {
        Ok(_) => (),
        Err(e) => println!("Error al cerrar el thread de la GUI: {:?}", e),
    }
}
