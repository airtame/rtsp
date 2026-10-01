use crate::message::{MessageError, MessageHeaders, Version};

#[derive(Debug)]
pub struct Response {
    version: Version,
    status_code: u16,
    reason_phrase: String,
    headers: MessageHeaders,
    body: tokio_util::bytes::Bytes,
}

impl Response {
    pub(crate) fn new(
        start_line: &[u8],
        headers: MessageHeaders,
        body: tokio_util::bytes::Bytes,
    ) -> Result<Self, MessageError> {
        let start_line = std::str::from_utf8(start_line).map_err(|_| {
            MessageError::InvalidEncoding(String::from_utf8_lossy(start_line).into_owned())
        })?;
        let invalid_status_line = || MessageError::InvalidStatusLine(start_line.to_owned());

        let (version, rest) = start_line.trim().split_once(' ').ok_or_else(invalid_status_line)?;
        let rest = rest.trim_start();
        let (status_code, reason_phrase) = rest.split_once(' ').unwrap_or((rest, ""));

        let version = version.parse::<Version>().map_err(|_| invalid_status_line())?;
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
