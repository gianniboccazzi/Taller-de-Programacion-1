use std::fmt;

#[derive(Debug, PartialEq)]
pub enum GitrError {
    InputError,
    FileCreationError(String),
    FileWriteError(String),
    FileDeletionError(String),
    ObjectNotFound(String),
    FileReadError(String),
    FileDeleteError(String),
    NoCommitExisting(String),
    NoHead,
    AlreadyInitialized,
    NoRepository,
    InvalidArgumentError(String, String),
    LogError,
    CompressionError,
    TimeError,
    InvalidTreeError,
    InvalidCommitError,
    InvalidTagError,
    ConnectionError,
    SocketError(String, String),
    PackFileError(String, String),
    BranchNonExistsError(String),
    BranchAlreadyExistsError(String),
    DeleteCurrentBranchError(String),
    TagAlreadyExistsError(String),
    TagNonExistsError(String),
    PullRequestWriteError,
    PullRequestReadError,
    PushError(String),
    BranchNotFound,
}

impl fmt::Display for GitrError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InputError => write!(f, "Error en la entrada de comandos"),
            Self::BranchNonExistsError(branch) => {
                write!(f, "ERROR: branch '{}' no encontrada.", branch)
            }
            Self::FileDeletionError(fun) => {
                write!(f, "ERROR: En la funcion {} falló una eliminación", fun)
            }
            Self::FileCreationError(path) => {
                write!(f, "ERROR: No se pudo crear el archivo {}", path)
            }
            Self::FileWriteError(path) => {
                write!(f, "ERROR: No se pudo escribir el archivo {}", path)
            }
            Self::FileDeleteError(path) => {
                write!(f, "ERROR: No se pudo borrar el archivo {}", path)
            }
            Self::ObjectNotFound(obj) => write!(f, "ERROR: No se encontro el objeto {}", obj),
            Self::FileReadError(path) => write!(f, "ERROR: No se pudo leer el archivo {}", path),
            Self::BranchAlreadyExistsError(branch) => {
                write!(f, "error: la branch '{}' ya existe.", branch)
            }
            Self::NoHead => write!(f, "ERROR: No se encontro HEAD"),
            Self::AlreadyInitialized => write!(f, "ERROR: El repositorio ya esta inicializado"),
            Self::NoRepository => write!(f, "ERROR: No se encontro el repositorio"),
            Self::NoCommitExisting(brch) => {
                write!(f, "ERROR: nombre de objeto no valido: '{}'", brch)
            }
            Self::LogError => write!(f, "ERROR: No se pudo escribir en el log"),
            Self::CompressionError => write!(f, "ERROR: No se pudo comprimir el archivo"),
            Self::InvalidArgumentError(got, usage) => write!(
                f,
                "ERROR: Argumentos invalidos.\n    Recibi: {}\n    Uso: {}\n",
                got, usage
            ),
            Self::TimeError => write!(f, "ERROR: No se pudo obtener el tiempo actual"),
            Self::InvalidTreeError => write!(f, "ERROR: El arbol no es valido"),
            Self::InvalidCommitError => write!(f, "ERROR: El commit no es valido"),
            Self::ConnectionError => write!(f, "ERROR: No se pudo conectar al servidor"),
            Self::InvalidTagError => write!(f, "ERROR: La tag no es valida"),
            Self::SocketError(origin_function, info) => write!(
                f,
                "SocketError en la funcion {}. Info: {}",
                origin_function, info
            ),
            Self::PackFileError(origin_function, info) => write!(
                f,
                "PackFileError en la funcion {}. Info: {}",
                origin_function, info
            ),
            Self::TagAlreadyExistsError(tag) => write!(f, "ERROR: tag '{}' ya existe", tag),
            Self::TagNonExistsError(tag) => write!(f, "ERROR: tag '{}' no encontrado", tag),
            Self::DeleteCurrentBranchError(branch) => write!(
                f,
                "ERROR: No se puede borrar branch '{}': HEAD apunta ahi",
                branch
            ),
            Self::PullRequestWriteError => write!(f, "ERROR: No se pudo escribir el PR en el server"),
            Self::PullRequestReadError => write!(f, "ERROR: No se pudo leer el PR del server"),
            Self::PushError(info) => write!(f, "ERROR: No se pudo hacer push. Info: {}", info),
            Self::BranchNotFound => write!(f, "ERROR: No se encontro la branch"),

        }
    }
}
