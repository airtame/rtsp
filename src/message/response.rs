use crate::message::{MessageError, MessageHeaders, ParsingMode, StatusCode, Version};

const CSEQ: &str = "CSeq";

#[derive(Debug)]
pub struct Response {
    version: Version,
    status_code: StatusCode,
    headers: MessageHeaders,
    body: tokio_util::bytes::Bytes,
}

impl Response {
    pub fn new(version: Version, status_code: StatusCode) -> Self {
        assert!(
            !status_code.reason_phrase().contains(['\r', '\n']),
            "invalid reason phrase: {:?}",
            status_code.reason_phrase()
        );

        Self {
            version,
            status_code,
            headers: MessageHeaders::default(),
            body: tokio_util::bytes::Bytes::new(),
        }
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
        let code = status_code.parse::<u16>().map_err(|_| invalid_status_line())?;
        let status_code = StatusCode::try_from(code)
            .unwrap_or_else(|code| StatusCode::Extension(code, reason_phrase.trim().to_owned()));

        Ok(Self { version, status_code, headers, body })
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn status_code(&self) -> &StatusCode {
        &self.status_code
    }

    pub fn headers(&self) -> &MessageHeaders {
        &self.headers
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub(crate) fn encode(&self, dst: &mut tokio_util::bytes::BytesMut) {
        use std::fmt::Write as _;

        write!(
            dst,
            "{} {:03} {}\r\n",
            self.version,
            self.status_code.code(),
            self.status_code.reason_phrase()
        )
        .expect("writing to BytesMut can't fail");

        self.headers.encode(self.body.len(), dst);
        dst.extend_from_slice(b"\r\n");
        dst.extend_from_slice(&self.body);
    }
}

impl std::fmt::Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let reason_phrase = self.status_code.reason_phrase();
        write!(f, "{} {:03}", self.version, self.status_code.code())?;
        if !reason_phrase.is_empty() {
            write!(f, " {}", reason_phrase.escape_debug())?;
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
