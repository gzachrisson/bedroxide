use std::{fmt, io, result};
use raknet::{channel::SendError, Command};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
/// Enum with the possible Bedroxide errors.
pub enum Error {
    CommandError(String),
    /// A wrapper around an IO error.
    IoError(io::Error),
    /// A wrapper around a RakNet error.
    RakNetError(raknet::Error),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::CommandError(s) => write!(f, "Could not send the command: {:?}", s),
            Error::IoError(e) => write!(f, "An IO error occurred: {:?}", e),
            Error::RakNetError(err) => write!(f, "A RakNet error occurred: {:?}", err),
        }
    }
}

impl From<raknet::Error> for Error {
    fn from(error: raknet::Error) -> Self {
        Error::RakNetError(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::IoError(error)
    }
}

impl From<SendError<Command>> for Error {
    fn from(error: SendError<Command>) -> Self {
        Error::CommandError(error.to_string())
    }
}
