# 23C2-La-Liga-De-La-Rusticia
Repo for Rust Taller De Programacion 1 FIUBA

## Como correr el proyecto
Clonar el repo:
```bash
git clone git@github.com:taller-1-fiuba-rust/23C2-La-Liga-De-La-Rusticia.git rusticia
cd rusticia/gitr
```
Iniciar el Server:
```bash
cargo run --bin server
```

Iniciar el Cliente(en una terminal diferente al server):
```bash
cargo run --bin client <nombre-cliente>
```

## Problemas conocidos
En caso de tener problemas con Gtk, "version `GLIBCXX_3.4.29' not found" 
o similares, es  necesario ejecutar el siguiente comando, 
y luego volver a iniciar el cliente:
```bash
unset GTK_PATH
```
