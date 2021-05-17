use std::{fmt, io, result, string};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
/// Enum with the possible errors that can happen.
pub enum Error {
    /// A wrapper around an IO error.
    IoError(std::io::Error),
    /// An error happened while reading.
    ReadError(ReadError),
    /// An error happened while writing.
    WriteError(WriteError),
    /// An unknown message ID was received.
    UnknownMessageId(u8),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IoError(err) => write!(f, "An IO error occurred: {:?}", err),
            Error::ReadError(err) => write!(f, "Error while reading: {:?}", err),
            Error::WriteError(err) => write!(f, "Error while writing: {:?}", err),
            Error::UnknownMessageId(id) => write!(f, "Received an unknown message ID: {:?}", id),
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
        Error::ReadError(ReadError::InvalidString(error))
    }
}

impl From<ReadError> for Error {
    fn from(error: ReadError) -> Self {
        Error::ReadError(error)
    }
}

impl From<WriteError> for Error {
    fn from(error: WriteError) -> Self {
        Error::WriteError(error)
    }
}

#[derive(Debug)]
pub enum ReadError {
    /// The read value is not the same as the compare value.
    CompareFailed,
    /// The header was invalid.
    InvalidHeader,
    /// The IP version read was not 4 or 6.
    InvalidIpVersion,
    /// The read Offline Message ID was invalid.
    InvalidOfflineMessageId,
    /// A string was incorrectly encoded.
    InvalidString(string::FromUtf8Error),
    /// Not all bytes could be read.
    NotAllBytesRead(usize),
    /// The read zero padding was longer than allowed.
    TooLongZeroPadding,    
}

impl std::error::Error for ReadError {}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadError::CompareFailed => write!(f, "Read data is not the same as the compare value."),
            ReadError::InvalidHeader => write!(f, "Read invalid header."),
            ReadError::InvalidIpVersion => write!(f, "Received invalid IP version."),
            ReadError::InvalidOfflineMessageId => write!(f, "Received invalid Offline Message ID."),
            ReadError::InvalidString(err) => write!(f, "Could not parse string: {:?}", err),
            ReadError::NotAllBytesRead(c) => write!(f, "Could not read all bytes. Bytes read: {}", c),
            ReadError::TooLongZeroPadding => write!(f, "The read zero padding was longer than allowed."),
        }
    }
}

#[derive(Debug)]
pub enum WriteError {
    /// The header was invalid.
    InvalidHeader,
    /// Not all bytes could be written.
    NotAllBytesWritten(usize),
    /// Payload was too large.
    PayloadTooLarge,
    /// There were more ack/nack ranges in a
    /// datagram than what can fit into an u16.
    TooManyRanges,
}

impl std::error::Error for WriteError {}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WriteError::InvalidHeader => write!(f, "The header in invalid."),
            WriteError::NotAllBytesWritten(c) => write!(f, "Could not write all bytes. Bytes written: {}", c),
            WriteError::PayloadTooLarge => write!(f, "Payload too large."),
            WriteError::TooManyRanges => write!(f, "Too many acknowledgement ranges in datagram."),
        }
    }
}
