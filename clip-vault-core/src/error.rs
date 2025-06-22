use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Bincode(Box<bincode::ErrorKind>),
    Sqlite(rusqlite::Error),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Bincode(e) => Some(&**e),
            Error::Sqlite(e) => Some(e),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {e}"),
            Error::Bincode(e) => write!(f, "bincode error: {e}"),
            Error::Sqlite(e) => write!(f, "sqlite error: {e}"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<bincode::Error> for Error {
    fn from(e: bincode::Error) -> Self {
        Self::Bincode(e)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}

impl From<std::time::SystemTimeError> for Error {
    fn from(e: std::time::SystemTimeError) -> Self {
        Self::Io(io::Error::other(e))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
