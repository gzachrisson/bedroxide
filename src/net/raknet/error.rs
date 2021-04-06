#[derive(Debug)]
pub enum RakNetError {
    IoError(std::io::Error),
    TooFewBytesWritten(usize),
    TooFewBytesRead(usize),
    StringParseError(std::string::FromUtf8Error)
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
