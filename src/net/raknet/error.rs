use crossbeam_channel::SendError;

use super::Command;

#[derive(Debug)]
pub enum RakNetError {
    IoError(std::io::Error),
    TooFewBytesWritten(usize),
    TooFewBytesRead(usize),
    StringParseError(std::string::FromUtf8Error),
    UnknownMessageId(u8),
    CommandError(String)
}

impl From<std::io::Error> for RakNetError {
    fn from(error: std::io::Error) -> Self {
        RakNetError::IoError(error)
    }
}

impl From<std::string::FromUtf8Error> for RakNetError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        RakNetError::StringParseError(error)
    }
}

impl From<SendError<Command>> for RakNetError {
    fn from(error: SendError<Command>) -> Self {
        RakNetError::CommandError(error.to_string())
    }
}