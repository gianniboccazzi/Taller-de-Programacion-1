use std::error::Error;
use std::fmt;
#[derive(Debug, PartialEq)]
/// Representa todos los tipos de error que puede tener el programa. Cada error tiene su salida correspondiente.
pub enum ErrorDeLecturaLaberinto {
    ErrorDeParseo,
    ElementoInvalido,
    ErrordeArchivo,
    PosicionBombaInvalida,
}

impl fmt::Display for ErrorDeLecturaLaberinto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorDeLecturaLaberinto::ErrorDeParseo => {
                write!(f, "ERROR: No se pudo identificar un número del tablero")
            }
            ErrorDeLecturaLaberinto::ElementoInvalido => {
                write!(f, "ERROR: Elemento Invalido en el laberinto leído")
            }
            ErrorDeLecturaLaberinto::ErrordeArchivo => {
                write!(f, "ERROR: No se pudo leer el archivo correspondiente")
            }
            ErrorDeLecturaLaberinto::PosicionBombaInvalida => {
                write!(f, "ERROR: Posicion de bomba invalida")
            }
        }
    }
}
impl Error for ErrorDeLecturaLaberinto {}
