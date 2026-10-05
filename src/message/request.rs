use crate::message::{MessageError, MessageHeaders, ParsingMode, RequestMethod, Version};

const CSEQ: &str = "CSeq";

#[derive(Debug)]
pub struct Request {
    method: RequestMethod,
    uri: String,
    path: String,
    query: Option<String>,
    version: Version,
    headers: MessageHeaders,
    body: tokio_util::bytes::Bytes,
}

impl Request {
    pub fn new(method: RequestMethod, uri: impl Into<String>, version: Version) -> Self {
        let uri = uri.into();
        assert!(
            !uri.is_empty() && !uri.contains(|c: char| c.is_whitespace() || c.is_control()),
            "invalid request URI: {uri:?}"
        );
        let (path, query) = Self::parse_uri(&uri);

        Self {
            method,
            uri,
            path,
            query,
            version,
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

    pub(crate) fn with_cseq(mut self, cseq: u32) -> Self {
        self.headers.remove(CSEQ);
        self.headers.append(CSEQ.to_owned(), cseq.to_string());
        self
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
        let invalid_request_line = || MessageError::InvalidRequestLine(start_line.to_owned());

        let mut parts = start_line.split_ascii_whitespace();
        let (Some(method), Some(uri), Some(version), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(invalid_request_line());
        };

        let Ok(method) = method.parse::<RequestMethod>();
        let (path, query) = Self::parse_uri(uri);
        let version = version.parse::<Version>().map_err(|_| invalid_request_line())?;
        if parsing_mode == ParsingMode::Strict && matches!(version, Version::Other(_)) {
            return Err(invalid_request_line());
        }
        if parsing_mode == ParsingMode::Strict && headers.get(CSEQ).is_none() {
            return Err(MessageError::MissingHeader(CSEQ.to_owned()));
        }

        Ok(Self { method, uri: uri.to_owned(), path, query, version, headers, body })
    }

    pub fn method(&self) -> &RequestMethod {
        &self.method
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn headers(&self) -> &MessageHeaders {
        &self.headers
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub(crate) fn encode(&self, dst: &mut tokio_util::bytes::BytesMut) {
        use std::fmt::Write as _;

        write!(dst, "{} {} {}\r\n", self.method, self.uri, self.version)
            .expect("writing to BytesMut can't fail");

        self.headers.encode(self.body.len(), dst);
        dst.extend_from_slice(b"\r\n");
        dst.extend_from_slice(&self.body);
    }

    fn parse_uri(uri: &str) -> (String, Option<String>) {
        let uri = uri.split_once('#').map_or(uri, |(before_fragment, _)| before_fragment);

        let path_and_query = match uri.find("://") {
            Some(scheme_end) if !uri[..scheme_end].contains(['/', '?']) => {
                let after_scheme = &uri[scheme_end + 3..];
                after_scheme.find(['/', '?']).map_or("", |path_start| &after_scheme[path_start..])
            }
            _ => uri,
        };

        let (path, query) = match path_and_query.split_once('?') {
            Some((path, query)) => (path, Some(query.to_owned())),
            None => (path_and_query, None),
        };

        (if path.is_empty() { "/" } else { path }.to_owned(), query)
    }
}

impl std::fmt::Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.method.to_string().escape_debug(),
            self.uri.escape_debug(),
            self.version
        )?;
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
#[path = "test/request_unittests.rs"]
mod tests;
