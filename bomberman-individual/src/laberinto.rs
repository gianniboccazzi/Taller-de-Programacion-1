use crate::error_laberinto::ErrorDeLecturaLaberinto;
use crate::piezas::Piezas;
use std::fs::File;
use std::io::Write;
use std::path::Path;
#[derive(Debug)]
/// Representa el laberinto a crear. El mismo está conformado por una matriz de "Piezas".
pub struct Laberinto {
    tamaño: usize,
    tablero: Vec<Vec<Piezas>>,
}

impl Laberinto {
    /// Recibe una matriz de Strings y dependiendo cada espacio, crea cada pieza en su lugar correspondiente.
    pub fn crear(
        tamaño: usize,
        tablero: Vec<Vec<String>>,
    ) -> Result<Self, ErrorDeLecturaLaberinto> {
        let laberinto_armado = Laberinto::interpretar_laberinto(tablero)?;
        Ok(Laberinto {
            tamaño,
            tablero: laberinto_armado,
        })
    }

    fn interpretar_laberinto(
        tablero: Vec<Vec<String>>,
    ) -> Result<Vec<Vec<Piezas>>, ErrorDeLecturaLaberinto> {
        let mut laberinto: Vec<Vec<Piezas>> = Vec::new();
        for fila in &tablero {
            let mut fila_laberinto: Vec<Piezas> = Vec::new();
            for elemento in fila {
                fila_laberinto.push(Piezas::crear(elemento.to_string())?);
            }
            laberinto.push(fila_laberinto);
        }
        Ok(laberinto)
    }

    /// Recibe las coordenadas de una bomba, y la detona, modificando los lugares correspondientes.
    pub fn detonar_bomba(&mut self, x: usize, y: usize) -> Result<(), ErrorDeLecturaLaberinto> {
        if let Piezas::Bomba { alcance, traspasa } = self.tablero[x][y] {
            self.tablero[x][y] = Piezas::Espacio;
            self.direccionar_explosion(x, y, alcance, traspasa);
            Ok(())
        } else {
            Err(ErrorDeLecturaLaberinto::PosicionBombaInvalida)
        }
    }

    fn direccionar_explosion(&mut self, x: usize, y: usize, alcance: u32, traspasa: bool) {
        let mut enemigos_golpeados: Vec<(usize, usize)> = Vec::new();
        let mut desvios: Vec<(String, u32, usize, usize)> = Vec::new();
        self.direccion_up(
            x,
            y,
            alcance,
            traspasa,
            &mut enemigos_golpeados,
            &mut desvios,
        );
        self.direccion_left(
            x,
            y,
            alcance,
            traspasa,
            &mut enemigos_golpeados,
            &mut desvios,
        );
        self.direccion_right(
            x,
            y,
            alcance,
            traspasa,
            &mut enemigos_golpeados,
            &mut desvios,
        );
        self.direccion_down(
            x,
            y,
            alcance,
            traspasa,
            &mut enemigos_golpeados,
            &mut desvios,
        );
        while !desvios.is_empty() {
            if let Some((direc, des_alcance, des_x, des_y)) = desvios.pop() {
                self.hacer_desvios(
                    (des_x, des_y),
                    des_alcance,
                    traspasa,
                    &mut enemigos_golpeados,
                    &direc,
                    &mut desvios,
                )
            }
        }
    }

    fn direccion_left(
        &mut self,
        x: usize,
        y: usize,
        alcance: u32,
        traspasa: bool,
        golpeados: &mut Vec<(usize, usize)>,
        desvios: &mut Vec<(String, u32, usize, usize)>,
    ) {
        for i in 1..=alcance {
            if (y as i32 - i as i32) < 0 {
                break;
            }
            let res =
                self.check_elemento(x, y - i as usize, alcance - i, traspasa, golpeados, desvios);
            if !res {
                break;
            }
        }
    }

    fn direccion_right(
        &mut self,
        x: usize,
        y: usize,
        alcance: u32,
        traspasa: bool,
        golpeados: &mut Vec<(usize, usize)>,
        desvios: &mut Vec<(String, u32, usize, usize)>,
    ) {
        for i in 1..=alcance {
            if y as u32 + i >= self.tamaño as u32 {
                break;
            }
            let res =
                self.check_elemento(x, y + i as usize, alcance - i, traspasa, golpeados, desvios);
            if !res {
                break;
            }
        }
    }

    fn direccion_up(
        &mut self,
        x: usize,
        y: usize,
        alcance: u32,
        traspasa: bool,
        golpeados: &mut Vec<(usize, usize)>,
        desvios: &mut Vec<(String, u32, usize, usize)>,
    ) {
        for i in 1..=alcance {
            if (x as i32 - i as i32) < 0 {
                break;
            }
            let res =
                self.check_elemento(x - i as usize, y, alcance - i, traspasa, golpeados, desvios);
            if !res {
                break;
            }
        }
    }

    fn direccion_down(
        &mut self,
        x: usize,
        y: usize,
        alcance: u32,
        traspasa: bool,
        golpeados: &mut Vec<(usize, usize)>,
        desvios: &mut Vec<(String, u32, usize, usize)>,
    ) {
        for i in 1..=alcance {
            if x as u32 + i >= self.tamaño as u32 {
                break;
            }
            let res =
                self.check_elemento(x + i as usize, y, alcance - i, traspasa, golpeados, desvios);
            if !res {
                break;
            }
        }
    }

    fn check_elemento(
        &mut self,
        x: usize,
        y: usize,
        alcance: u32,
        traspasa: bool,
        golpeados: &mut Vec<(usize, usize)>,
        desvios: &mut Vec<(String, u32, usize, usize)>,
    ) -> bool {
        if let Some(fila) = self.tablero.get(x) {
            if let Some(elemento) = fila.get(y) {
                match elemento {
                    Piezas::Bomba { .. } => {
                        let _ = self.detonar_bomba(x, y);
                    }
                    Piezas::Enemigo { vidas } => {
                        if golpeados.contains(&(x, y)) {
                            return true;
                        }
                        if *vidas == 1 {
                            self.tablero[x][y] = Piezas::Espacio
                        } else {
                            golpeados.push((x, y));
                            self.tablero[x][y] = Piezas::Enemigo { vidas: vidas - 1 };
                        }
                    }
                    Piezas::Desvio { direc } => {
                        desvios.push((direc.to_string(), alcance, x, y));
                        return false;
                    }
                    Piezas::Roca => return traspasa,
                    Piezas::Pared => return false,
                    _ => return true,
                }
            }
        }
        true
    }

    fn hacer_desvios(
        &mut self,
        coords: (usize, usize),
        alcance: u32,
        traspasa: bool,
        golpeados: &mut Vec<(usize, usize)>,
        direc: &str,
        desvios: &mut Vec<(String, u32, usize, usize)>,
    ) {
        match direc {
            "U" => self.direccion_up(coords.0, coords.1, alcance, traspasa, golpeados, desvios),
            "D" => self.direccion_down(coords.0, coords.1, alcance, traspasa, golpeados, desvios),
            "R" => self.direccion_right(coords.0, coords.1, alcance, traspasa, golpeados, desvios),
            "L" => self.direccion_left(coords.0, coords.1, alcance, traspasa, golpeados, desvios),
            _ => (),
        }
    }
    /// Escribe en un archivo el resultado final del laberinto.     
    pub fn escribir_laberinto_final(&self, path_out: &str, archivo_entrada: &str) {
        let mut nombre_archivo: &str = "";
        if let Some(path) = Path::new(archivo_entrada).file_name() {
            if let Some(nombre_archivo_str) = path.to_str() {
                nombre_archivo = nombre_archivo_str;
            }
        }
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
        let res = self.to_string();
        let res_escritura = archivo.write_all(res.as_bytes());
        match res_escritura {
            Ok(_) => (),
            Err(_) => {
                println!("No se pudo escribir el archivo output");
            }
        }
    }
}

impl ToString for Laberinto {
    fn to_string(&self) -> String {
        let mut res: String = String::new();
        for fila in self.tablero.iter() {
            let vector_duplicado: Vec<String> = fila.iter().map(|elem| elem.to_string()).collect();
            let vec_join = vector_duplicado.join(" ");
            let salto_agregado = format!("{}\n", vec_join);
            res.push_str(&salto_agregado);
        }
        res
    }
}

#[test]
pub fn test_creacion_lab() {
    let entrada: Vec<Vec<String>> = vec![
        vec!["_".to_string(), "B2".to_string()],
        vec!["S2".to_string(), "DL".to_string()],
    ];
    let res: Laberinto = Laberinto::crear(entrada.len(), entrada).unwrap();
    let res_correcto: Vec<Vec<Piezas>> = vec![
        vec![
            Piezas::Espacio,
            Piezas::Bomba {
                alcance: 2,
                traspasa: false,
            },
        ],
        vec![
            Piezas::Bomba {
                alcance: 2,
                traspasa: true,
            },
            Piezas::Desvio {
                direc: "L".to_string(),
            },
        ],
    ];
    assert_eq!(res.tablero, res_correcto)
}

#[test]
pub fn test_creacion_lab_bomba_negativa() {
    let entrada: Vec<Vec<String>> = vec![
        vec!["_".to_string(), "B-1".to_string()],
        vec!["S-2".to_string(), "DL".to_string()],
    ];
    let res = Laberinto::crear(entrada.len(), entrada);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), ErrorDeLecturaLaberinto::ErrorDeParseo);
}

#[test]
pub fn test_creacion_lab_elemento_invalido() {
    let entrada: Vec<Vec<String>> = vec![
        vec!["_".to_string(), "T".to_string()],
        vec!["O".to_string(), "DL".to_string()],
    ];
    let res = Laberinto::crear(entrada.len(), entrada);
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), ErrorDeLecturaLaberinto::ElementoInvalido);
}
#[test]
pub fn test_detonar_bomba() {
    let entrada: Vec<Vec<String>> = vec![
        vec!["B1".to_string(), "B2".to_string()],
        vec!["S2".to_string(), "DL".to_string()],
    ];
    let mut res: Laberinto = Laberinto::crear(entrada.len(), entrada).unwrap();
    res.detonar_bomba(0, 0).unwrap();
    let res_correcto: Vec<Vec<Piezas>> = vec![
        vec![Piezas::Espacio, Piezas::Espacio],
        vec![
            Piezas::Espacio,
            Piezas::Desvio {
                direc: "L".to_string(),
            },
        ],
    ];
    assert_eq!(res.tablero, res_correcto);
}

#[test]
pub fn test_to_string() {
    let entrada: Vec<Vec<String>> = vec![
        vec!["B1".to_string(), "B2".to_string()],
        vec!["S2".to_string(), "DL".to_string()],
    ];
    let res: Laberinto = Laberinto::crear(entrada.len(), entrada).unwrap();
    let res_string = res.to_string();
    let res_correcto = "B1 B2\nS2 DL\n".to_string();
    assert_eq!(res_correcto, res_string);
}

#[test]
pub fn test_escritura_archivoin() {
    let path_out = ".";
    let archivo_entrada = "test_escritura.txt";
    let entrada: Vec<Vec<String>> = vec![
        vec!["B1".to_string(), "B2".to_string()],
        vec!["S2".to_string(), "DL".to_string()],
    ];
    let res_esperado: Laberinto = Laberinto::crear(entrada.len(), entrada).unwrap();
    res_esperado.escribir_laberinto_final(path_out, archivo_entrada);
    assert!(Path::new(&archivo_entrada).exists());
    let archivo_origen = File::open(archivo_entrada).expect("Error al leer archivo");
    let lab_string = crate::leer_laberinto(archivo_origen).unwrap();
    let res = Laberinto::crear(lab_string.len(), lab_string).unwrap();
    assert_eq!(res_esperado.to_string(), res.to_string());
    std::fs::remove_file(archivo_entrada).expect("Error al eliminar el archivo");
}

#[test]
pub fn test_escritura_directorio_in() {
    let path_out = ".";
    let archivo_entrada = "prueba/test_escritura2.txt";
    let path_esperado = "test_escritura2.txt";
    let entrada: Vec<Vec<String>> = vec![
        vec!["B1".to_string(), "B2".to_string()],
        vec!["S2".to_string(), "DL".to_string()],
    ];
    let res_esperado: Laberinto = Laberinto::crear(entrada.len(), entrada).unwrap();
    res_esperado.escribir_laberinto_final(path_out, archivo_entrada);
    assert!(Path::new(&path_esperado).exists());
    let archivo_origen = File::open(path_esperado).expect("Error al leer archivo");
    let lab_string = crate::leer_laberinto(archivo_origen).unwrap();
    let res = Laberinto::crear(lab_string.len(), lab_string).unwrap();
    assert_eq!(res_esperado.to_string(), res.to_string());
    std::fs::remove_file(path_esperado).expect("Error al eliminar el archivo");
}
