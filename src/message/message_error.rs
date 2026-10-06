#[derive(Debug)]
#[non_exhaustive]
pub enum MessageError {
    Io(std::io::Error),
    InvalidContentLength(String),
    InvalidEncoding(String),
    InvalidHeader(String),
    InvalidRequestLine(String),
    InvalidStatusLine(String),
    MissingHeader(String),
}

impl std::fmt::Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::InvalidContentLength(value) => write!(f, "invalid Content-Length: {value:?}"),
            Self::InvalidEncoding(text) => write!(f, "invalid encoding, expected UTF-8: {text:?}"),
            Self::InvalidHeader(line) => write!(f, "invalid header: {line:?}"),
            Self::InvalidRequestLine(line) => write!(f, "invalid request line: {line:?}"),
            Self::InvalidStatusLine(line) => write!(f, "invalid status line: {line:?}"),
            Self::MissingHeader(name) => write!(f, "missing header: {name:?}"),
        }
    }
}

impl std::error::Error for MessageError {}

impl From<std::io::Error> for MessageError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

#[cfg(test)]
#[path = "test/message_error_unittests.rs"]
mod tests;
