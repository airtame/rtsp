use crate::message::{MessageError, MessageHeaders, Request, Response};

#[derive(Debug)]
pub(crate) enum Message {
    Request(Request),
    Response(Response),
}

impl Message {
    pub(crate) fn new(
        start_line: &[u8],
        headers: MessageHeaders,
        body: tokio_util::bytes::Bytes,
    ) -> Result<Self, MessageError> {
        let first_word =
            start_line.trim_ascii_start().split(|&byte| byte == b' ' || byte == b'\r').next();
        if first_word.is_some_and(|word| word.contains(&b'/')) {
            Response::new(start_line, headers, body).map(Self::Response)
        } else {
            Request::new(start_line, headers, body).map(Self::Request)
        }
    }

    pub(crate) fn encode(&self, dst: &mut tokio_util::bytes::BytesMut) {
        match self {
            Self::Request(request) => request.encode(dst),
            Self::Response(response) => response.encode(dst),
        }
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request(request) => write!(f, "{request}"),
            Self::Response(response) => write!(f, "{response}"),
        }
    }
}

#[cfg(test)]
#[path = "test/message_unittests.rs"]
mod tests;
