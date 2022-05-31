use std::{fmt, io, result, str::Utf8Error};
use raknet::{channel::SendError, Command};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
/// Enum with the possible Bedroxide errors.
pub enum Error {
    CommandError(String),
    /// A wrapper around an IO error.
    IoError(io::Error),
    /// A wrapper around an Utf8Error.
    Utf8Error(Utf8Error),
    /// A wrapper around a RakNet error.
    RakNetError(raknet::Error),
    /// The VarInt number was too large to fit into the desired type.
    VarIntTooLarge,
    /// Not all bytes could be read.
    NotAllBytesRead,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::CommandError(s) => write!(f, "Could not send the command: {:?}", s),
            Error::IoError(e) => write!(f, "An IO error occurred: {:?}", e),
            Error::Utf8Error(e) => write!(f, "An UTF8 conversion error occurred: {:?}", e),
            Error::RakNetError(e) => write!(f, "A RakNet error occurred: {:?}", e),
            Error::VarIntTooLarge => write!(f, "The VarInt number was too large to fit into the desired type."),
            Error::NotAllBytesRead => write!(f, "Not all bytes could be read."),
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

impl From<Utf8Error> for Error {
    fn from(error: Utf8Error) -> Self {
        Error::Utf8Error(error)
    }
}

impl From<SendError<Command>> for Error {
    fn from(error: SendError<Command>) -> Self {
        Error::CommandError(error.to_string())
    }
}
