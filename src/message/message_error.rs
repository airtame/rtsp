// TODO(atokodi): Rename to HeaderError? Have a separarate MessageError.
// TODO(atokodi): Simply use std::io::Error with ErrorKind::InvalidData as in version.rs?
#[derive(Debug)]
pub enum MessageError {
    Io(std::io::Error),
    InvalidContentLength(String),
    InvalidEncoding(String),
    InvalidHeader(String),
    InvalidRequestLine(String),
    InvalidStatusLine(String),
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
        }
    }
}

impl From<std::io::Error> for MessageError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

#[cfg(test)]
#[path = "test/message_error_unittests.rs"]
mod tests;
