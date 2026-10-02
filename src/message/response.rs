use crate::message::{MessageError, MessageHeaders, ParsingMode, StatusCode, Version};

const CSEQ: &str = "CSeq";

#[derive(Debug)]
pub struct Response {
    version: Version,
    status_code: u16,
    reason_phrase: String,
    headers: MessageHeaders,
    body: tokio_util::bytes::Bytes,
}

impl Response {
    pub fn new(version: Version, status_code: StatusCode) -> Self {
        Self {
            version,
            status_code: status_code.code(),
            reason_phrase: status_code.reason_phrase().to_owned(),
            headers: MessageHeaders::default(),
            body: tokio_util::bytes::Bytes::new(),
        }
    }

    pub fn with_reason_phrase(mut self, reason_phrase: impl Into<String>) -> Self {
        self.reason_phrase = reason_phrase.into();
        self
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.append(name.into(), value.into());
        self
    }

    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = tokio_util::bytes::Bytes::from(body.into());
        self
    }

    pub(crate) fn with_replaced_header(mut self, name: &str, value: &str) -> Self {
        self.headers.remove(name);
        self.headers.append(name.to_owned(), value.to_owned());
        self
    }

    pub(crate) fn with_cseq(self, cseq: &str) -> Self {
        if cseq.is_empty() || !cseq.bytes().all(|byte| byte.is_ascii_digit()) {
            return self;
        }

        self.with_replaced_header(CSEQ, cseq)
    }

    pub(crate) fn parse(
        start_line: &[u8],
        headers: MessageHeaders,
        body: tokio_util::bytes::Bytes,
        parsing_mode: ParsingMode,
    ) -> Result<Self, MessageError> {
        let start_line = std::str::from_utf8(start_line).map_err(|_| {
            MessageError::InvalidEncoding(String::from_utf8_lossy(start_line).into_owned())
        })?;
        let invalid_status_line = || MessageError::InvalidStatusLine(start_line.to_owned());

        let (version, rest) = start_line.trim().split_once(' ').ok_or_else(invalid_status_line)?;
        let rest = rest.trim_start();
        let (status_code, reason_phrase) = rest.split_once(' ').unwrap_or((rest, ""));

        let version = version.parse::<Version>().map_err(|_| invalid_status_line())?;
        if parsing_mode == ParsingMode::Strict && matches!(version, Version::Other(_)) {
            return Err(invalid_status_line());
        }
        if status_code.len() != 3 || !status_code.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(invalid_status_line());
        }
        let status_code = status_code.parse().map_err(|_| invalid_status_line())?;

        Ok(Self {
            version,
            status_code,
            reason_phrase: reason_phrase.trim().to_owned(),
            headers,
            body,
        })
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn status_code(&self) -> u16 {
        self.status_code
    }

    pub fn reason_phrase(&self) -> &str {
        &self.reason_phrase
    }

    pub fn headers(&self) -> &MessageHeaders {
        &self.headers
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    // TODO(atokodi): Always write Content-Length (0 for an empty body), except for 1xx, 204 and
    // 304 responses. Without it an HTTP client in lenient mode reads the body until the
    // connection closes, so it waits for the idle timeout.
    pub(crate) fn encode(&self, dst: &mut tokio_util::bytes::BytesMut) {
        use std::fmt::Write as _;

        write!(dst, "{} {:03} {}\r\n", self.version, self.status_code, self.reason_phrase)
            .expect("writing to BytesMut can't fail");

        self.headers.encode(self.body.len(), dst);
        dst.extend_from_slice(b"\r\n");
        dst.extend_from_slice(&self.body);
    }
}

impl std::fmt::Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:03}", self.version, self.status_code)?;
        if !self.reason_phrase.is_empty() {
            write!(f, " {}", self.reason_phrase.escape_debug())?;
        }
        if !self.headers.is_empty() {
            write!(f, "\n{}", self.headers)?;
        }
        if !self.body.is_empty() {
            writeln!(f)?;
            for line in String::from_utf8_lossy(&self.body).lines() {
                write!(f, "\n{}", line.escape_debug())?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "test/response_unittests.rs"]
mod tests;
