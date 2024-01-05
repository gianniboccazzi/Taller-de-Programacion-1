use crate::error_laberinto::ErrorDeLecturaLaberinto;
#[derive(PartialEq, Debug)]

/// Representa las opciones que tiene el laberinto
pub enum Piezas {
    Bomba { alcance: u32, traspasa: bool },
    Enemigo { vidas: u32 },
    Roca,
    Pared,
    Desvio { direc: String },
    Espacio,
}

impl Piezas {
    /// Crea la pieza correspondiente dependiendo del caracter que lea en el laberinto de entrada
    pub fn crear(valor: String) -> Result<Self, ErrorDeLecturaLaberinto> {
        let letra_inicial = obtener_letra_posicion(&valor, 0);
        if valor.len() == 1 {
            return match &letra_inicial[..] {
                "R" => Ok(Piezas::Roca),
                "W" => Ok(Piezas::Pared),
                "_" => Ok(Piezas::Espacio),
                _ => Err(ErrorDeLecturaLaberinto::ElementoInvalido {}),
            };
        }
        let segunda_letra = obtener_letra_posicion(&valor, 1);
        match &letra_inicial[..] {
            "B" => Piezas::crear_bomba(false, segunda_letra),
            "S" => Piezas::crear_bomba(true, segunda_letra),
            "F" => Piezas::crear_enemigo(segunda_letra),
            "D" => Ok(Piezas::Desvio {
                direc: segunda_letra,
            }),
            _ => Err(ErrorDeLecturaLaberinto::ErrorDeParseo),
        }
    }

    fn crear_bomba(traspasa: bool, alcance: String) -> Result<Self, ErrorDeLecturaLaberinto> {
        let numero = alcance.parse();
        match numero {
            Ok(num) => {
                if num > 0 {
                    Ok(Piezas::Bomba {
                        traspasa,
                        alcance: num,
                    })
                } else {
                    Err(ErrorDeLecturaLaberinto::ErrorDeParseo)
                }
            }
            Err(_) => Err(ErrorDeLecturaLaberinto::ErrorDeParseo),
        }
    }
    fn crear_enemigo(vidas: String) -> Result<Self, ErrorDeLecturaLaberinto> {
        let numero = vidas.parse();
        match numero {
            Ok(num) => {
                let a = 1..=3;
                if a.contains(&num) {
                    Ok(Piezas::Enemigo { vidas: num })
                } else {
                    Err(ErrorDeLecturaLaberinto::ErrorDeParseo)
                }
            }
            Err(_) => Err(ErrorDeLecturaLaberinto::ErrorDeParseo),
        }
    }
}

fn obtener_letra_posicion(texto: &str, pos: usize) -> String {
    if let Some(primer_caracter) = texto.chars().nth(pos) {
        primer_caracter.to_string()
    } else {
        String::new()
    }
}

impl ToString for Piezas {
    fn to_string(&self) -> String {
        match self {
            Piezas::Bomba { alcance, traspasa } => {
                if *traspasa {
                    format!("S{}", alcance)
                } else {
                    format!("B{}", alcance)
                }
            }
            Piezas::Enemigo { vidas } => format!("F{}", vidas),
            Piezas::Desvio { direc } => format!("D{}", direc),
            Piezas::Espacio => String::from("_"),
            Piezas::Pared => String::from("W"),
            Piezas::Roca => String::from("R"),
        }
    }
}

#[test]
pub fn test_crear_piezas() {
    let casos: Vec<&str> = vec!["R", "W", "_", "B2", "S3", "DU", "F2"];
    let mut res: Vec<Piezas> = Vec::new();
    for elemento in casos {
        let pieza = Piezas::crear(elemento.to_string()).unwrap();
        res.push(pieza);
    }
    let caso_correcto: Vec<Piezas> = vec![
        Piezas::Roca,
        Piezas::Pared,
        Piezas::Espacio,
        Piezas::Bomba {
            alcance: 2,
            traspasa: false,
        },
        Piezas::Bomba {
            alcance: 3,
            traspasa: true,
        },
        Piezas::Desvio {
            direc: "U".to_string(),
        },
        Piezas::Enemigo { vidas: 2 },
    ];
    assert_eq!(res, caso_correcto);
}
#[test]
pub fn test_piezas_to_string() {
    let mut res: Vec<String> = Vec::new();
    let casos: Vec<Piezas> = vec![
        Piezas::Roca,
        Piezas::Pared,
        Piezas::Espacio,
        Piezas::Bomba {
            alcance: 2,
            traspasa: false,
        },
        Piezas::Bomba {
            alcance: 3,
            traspasa: true,
        },
        Piezas::Desvio {
            direc: "U".to_string(),
        },
        Piezas::Enemigo { vidas: 2 },
    ];
    for elemento in casos {
        let pieza_string = elemento.to_string();
        res.push(pieza_string);
    }
    let correcto: Vec<&str> = vec!["R", "W", "_", "B2", "S3", "DU", "F2"];
    assert_eq!(res, correcto);
}
