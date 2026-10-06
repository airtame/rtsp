#[derive(Debug)]
#[non_exhaustive]
pub enum RequestError {
    ConnectionClosed,
}

impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionClosed => write!(f, "connection closed before the response arrived"),
        }
    }
}

impl std::error::Error for RequestError {}

#[cfg(test)]
#[path = "test/request_error_unittests.rs"]
mod tests;
