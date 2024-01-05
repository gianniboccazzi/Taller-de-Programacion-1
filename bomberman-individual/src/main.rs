use std::env;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
mod error_laberinto;
mod laberinto;
mod piezas;
use crate::laberinto::Laberinto;
use error_laberinto::ErrorDeLecturaLaberinto;
const ARGS_NECESARIOS: usize = 5;
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < ARGS_NECESARIOS {
        println!("Argumentos insuficientes para ejecutar el programa");
        return;
    }
    let (nombre_archivo_in, path_out, num_x, num_y) = match valores_entrada(&args) {
        Ok((nombre, path, x, y)) => (nombre, path, x, y),
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };
    match ejecutar(nombre_archivo_in, path_out, num_x, num_y) {
        Ok(_) => (),
        Err(error) => {
            if error.is::<std::io::Error>() {
                escribir_error(
                    &ErrorDeLecturaLaberinto::ErrordeArchivo,
                    path_out,
                    nombre_archivo_in,
                )
            } else if let Some(error) = error.downcast_ref::<ErrorDeLecturaLaberinto>() {
                escribir_error(error, path_out, nombre_archivo_in)
            }
        }
    }
}

/// Recibe el nombre del archivo, la dirección donde debe escribir la salida, y la
/// coordenada donde debe detonar la bomba.
///
/// Abre el archivo, lee y crea el laberinto, detona la bomba y finalmente devuelve el resultado
/// en el path indicado.
fn ejecutar(
    nombre_archivo_in: &str,
    path_out: &str,
    num_x: i32,
    num_y: i32,
) -> Result<(), Box<dyn Error>> {
    let archivo_origen = File::open(nombre_archivo_in)?;
    let lab_string = leer_laberinto(archivo_origen)?;
    if num_x < 0
        || num_y < 0
        || num_x as usize >= lab_string.len()
        || num_y as usize >= lab_string.len()
    {
        let boxed_error: Box<dyn Error> = Box::new(ErrorDeLecturaLaberinto::PosicionBombaInvalida);
        return Err(boxed_error);
    }
    let mut laberinto_creado = Laberinto::crear(lab_string.len(), lab_string)?;
    laberinto_creado.detonar_bomba(num_y as usize, num_x as usize)?;
    laberinto_creado.escribir_laberinto_final(path_out, nombre_archivo_in);
    Ok(())
}

/// Recibe los comandos recibidos al ejecutar el programa, y devuelve
/// el nombre del archivo, la direccion donde será la salida del programa,
/// y las coordenadas donde debe detonarse la bomba.
/// Devuelve en una tupla los datos mencionados, siendo las coordenadas en usize.
fn valores_entrada(args: &[String]) -> Result<(&str, &str, i32, i32), &'static str> {
    let nombre_archivo_in = &args[1];
    let path_out = &args[2];
    let num_x = match args[3].parse::<i32>() {
        Ok(n) => n,
        Err(_) => {
            return Err("Error de parseo en coord_x");
        }
    };
    let num_y = match args[4].parse::<i32>() {
        Ok(n) => n,
        Err(_) => {
            return Err("Error de parseo en coord_y");
        }
    };
    Ok((nombre_archivo_in, path_out, num_x, num_y))
}

/// Recibe un error, una direccion y un nombre de archivo, y escribe el error en un archivo
/// con el nombre recibido, en la dirección definida.
fn escribir_error(error: &ErrorDeLecturaLaberinto, path_out: &str, nombre_archivo: &str) {
    let mut dir = format!("{}{}", path_out, nombre_archivo);
    if path_out == "." {
        dir = nombre_archivo.to_string();
    }
    let archivo_res = File::create(dir);
    let mut archivo = match archivo_res {
        Ok(escritura) => escritura,
        Err(_) => {
            println!("No se pudo escribir el archivo output");
            return;
        }
    };
    let salida = format!("{}", error);
    let res_escritura = archivo.write_all(salida.as_bytes());
    match res_escritura {
        Ok(_) => (),
        Err(_) => {
            println!("No se pudo escribir el archivo output");
        }
    }
}
/// Recibe un archivo, y devuelve una matriz con las piezas (aun en formato String) del
/// laberinto que se va a crear.
pub fn leer_laberinto(origen: File) -> io::Result<Vec<Vec<String>>> {
    let mut laberinto: Vec<Vec<String>> = Vec::new();
    let lector = BufReader::new(origen);
    for linea_resultado in lector.lines() {
        let linea = linea_resultado?;
        let fila: Vec<String> = linea.split_whitespace().map(String::from).collect();
        laberinto.push(fila);
    }
    Ok(laberinto)
}

#[test]
fn ejemplo_1_test() {
    let origen_string = "ejemplo_1.txt";
    let archivo_origen = File::open(origen_string).unwrap();
    let lab_string = leer_laberinto(archivo_origen).unwrap();
    let mut laberinto_creado = Laberinto::crear(lab_string.len(), lab_string).unwrap();
    laberinto_creado.detonar_bomba(0, 0).unwrap();
    let res = laberinto_creado.to_string();
    let res_correcto = "_ R R _ _ _ _\n_ W R W _ W _\n_ _ _ _ _ _ _\n_ W _ W _ W _\n_ _ _ _ _ _ _\n_ W _ W _ W _\n_ _ _ _ _ _ _\n";
    assert_eq!(res, res_correcto);
}

#[test]
fn ejemplo_2_test() {
    let origen_string = "ejemplo_2.txt";
    let archivo_origen = File::open(origen_string).unwrap();
    let lab_string = leer_laberinto(archivo_origen).unwrap();
    let mut laberinto_creado = Laberinto::crear(lab_string.len(), lab_string).unwrap();
    laberinto_creado.detonar_bomba(4, 2).unwrap();
    let res = laberinto_creado.to_string();
    let res_correcto = "_ _ _ _ _ _ _\n_ W _ W _ W _\n_ _ _ R F1 _ _\n_ W _ W R W _\n_ _ _ _ _ _ _\n_ W _ W _ W _\n_ _ _ _ _ _ B1\n";
    assert_eq!(res, res_correcto);
}

#[test]
fn ejemplo_3_test() {
    let origen_string = "ejemplo_3.txt";
    let archivo_origen = File::open(origen_string).unwrap();
    let lab_string = leer_laberinto(archivo_origen).unwrap();
    let mut laberinto_creado = Laberinto::crear(lab_string.len(), lab_string).unwrap();
    laberinto_creado.detonar_bomba(4, 0).unwrap();
    let res = laberinto_creado.to_string();
    let res_correcto = "_ _ _ _ _ _ _\n_ W _ W _ W _\n_ R R R _ _ _\n_ W _ W _ W _\n_ _ _ _ DU _ _\n_ W _ W _ W _\n_ _ _ _ _ _ _\n";
    assert_eq!(res, res_correcto);
}
#[test]
fn test_coordenada_negativa() {
    let entrada: Vec<Vec<String>> = vec![
        vec!["B3".to_string(), "B2".to_string()],
        vec!["S2".to_string(), "DL".to_string()],
    ];
    let lab: Laberinto = Laberinto::crear(entrada.len(), entrada).unwrap();
    let path_out = ".";
    let archivo_entrada = "test_escritura_neg.txt";
    lab.escribir_laberinto_final(path_out, archivo_entrada);
    match ejecutar(archivo_entrada, path_out, -1, -1) {
        Ok(_) => (),
        Err(error) => {
            if error.is::<std::io::Error>() {
                escribir_error(
                    &ErrorDeLecturaLaberinto::ErrordeArchivo,
                    path_out,
                    archivo_entrada,
                )
            } else if let Some(error) = error.downcast_ref::<ErrorDeLecturaLaberinto>() {
                escribir_error(error, path_out, archivo_entrada)
            }
        }
    }
    let mut archivo = File::open(archivo_entrada).unwrap();
    let mut contenido_archivo = String::new();
    archivo.read_to_string(&mut contenido_archivo).unwrap();
    let string_esperado = "ERROR: Posicion de bomba invalida";
    assert_eq!(contenido_archivo, string_esperado);
    std::fs::remove_file(archivo_entrada).expect("Error al eliminar el archivo");
}

#[test]
fn test_coordenada_mayor_a_tablero() {
    let entrada: Vec<Vec<String>> = vec![
        vec!["B3".to_string(), "B2".to_string(), "B3".to_string()],
        vec!["S2".to_string(), "B1".to_string(), "DU".to_string()],
        vec!["S2".to_string(), "DL".to_string(), "_".to_string()],
    ];
    let lab: Laberinto = Laberinto::crear(entrada.len(), entrada).unwrap();
    let path_out = ".";
    let archivo_entrada = "test_escritura_mayor.txt";
    lab.escribir_laberinto_final(path_out, archivo_entrada);
    match ejecutar(archivo_entrada, path_out, 100, 100) {
        Ok(_) => (),
        Err(error) => {
            if error.is::<std::io::Error>() {
                escribir_error(
                    &ErrorDeLecturaLaberinto::ErrordeArchivo,
                    path_out,
                    archivo_entrada,
                )
            } else if let Some(error) = error.downcast_ref::<ErrorDeLecturaLaberinto>() {
                escribir_error(error, path_out, archivo_entrada)
            }
        }
    }
    let mut archivo = File::open(archivo_entrada).unwrap();
    let mut contenido_archivo = String::new();
    archivo.read_to_string(&mut contenido_archivo).unwrap();
    let string_esperado = "ERROR: Posicion de bomba invalida";
    assert_eq!(contenido_archivo, string_esperado);
    std::fs::remove_file(archivo_entrada).expect("Error al eliminar el archivo");
}

#[test]
fn test_coordenada_sin_bomba() {
    let entrada: Vec<Vec<String>> = vec![
        vec!["DU".to_string(), "B2".to_string(), "B3".to_string()],
        vec!["S2".to_string(), "B1".to_string(), "DU".to_string()],
        vec!["S2".to_string(), "DL".to_string(), "_".to_string()],
    ];
    let lab: Laberinto = Laberinto::crear(entrada.len(), entrada).unwrap();
    let path_out = ".";
    let archivo_entrada = "test_escritura3.txt";
    lab.escribir_laberinto_final(path_out, archivo_entrada);
    match ejecutar(archivo_entrada, path_out, 0, 0) {
        Ok(_) => (),
        Err(error) => {
            if error.is::<std::io::Error>() {
                escribir_error(
                    &ErrorDeLecturaLaberinto::ErrordeArchivo,
                    path_out,
                    archivo_entrada,
                )
            } else if let Some(error) = error.downcast_ref::<ErrorDeLecturaLaberinto>() {
                escribir_error(error, path_out, archivo_entrada)
            }
        }
    }
    let mut archivo = File::open(archivo_entrada).unwrap();
    let mut contenido_archivo = String::new();
    archivo.read_to_string(&mut contenido_archivo).unwrap();
    let string_esperado = "ERROR: Posicion de bomba invalida";
    assert_eq!(contenido_archivo, string_esperado);
    std::fs::remove_file(archivo_entrada).expect("Error al eliminar el archivo");
}
