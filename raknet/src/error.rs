use std::{fmt, io, result, string};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
/// Enum with the possible errors that can happen.
pub enum Error {
    /// A wrapper around an IO error.
    IoError(std::io::Error),
    /// Not all bytes could be written.
    NotAllBytesWritten(usize),
    /// Not all bytes could be read.
    NotAllBytesRead(usize),
    /// A string could not be parsed.
    StringParseError(string::FromUtf8Error),
    /// An unknown message ID was received.
    UnknownMessageId(u8),
    /// Invalid data was read.
    InvalidData,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IoError(e) => write!(f, "An IO error occurred: {:?}", e),
            Error::NotAllBytesWritten(c) => write!(f, "Could not write all bytes. Bytes written: {}", c),
            Error::NotAllBytesRead(c) => write!(f, "Could not read all bytes. Bytes read: {}", c),
            Error::StringParseError(e) => write!(f, "Could not parse string: {:?}", e),
            Error::UnknownMessageId(id) => write!(f, "Received an unknown message ID: {:?}", id),
            Error::InvalidData => write!(f, "Received invalid data"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::IoError(error)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(error: string::FromUtf8Error) -> Self {
        Error::StringParseError(error)
    }
}
