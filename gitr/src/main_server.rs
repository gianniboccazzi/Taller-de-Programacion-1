use gitr::server::server_utils;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        println!("Usage: cargo run --bin server <port>");
        return;
    }
    let address = args[1].clone();
    println!("Server inicializado en {}", address);
    match server_utils::server_init(&address) {
        Ok(_) => println!("Server finalizado"),
        Err(e) => println!("Error: {}", e),
    }
}
