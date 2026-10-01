use crate::message::MessageError;

const CRLF: &str = "\r\n";
const CONTENT_LENGTH: &str = "Content-Length";

#[derive(Debug, Default)]
pub struct MessageHeaders {
    fields: Vec<(String, String)>,
}

impl MessageHeaders {
    // TODO(atokodi): If there are multiple headers with the same name this will only return the
    // first one.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(field_name, _)| field_name.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    pub(crate) fn encode(&self, body_length: usize, dst: &mut tokio_util::bytes::BytesMut) {
        use std::fmt::Write as _;

        let mut wrote_content_length = false;
        for (name, value) in &self.fields {
            if !name.eq_ignore_ascii_case(CONTENT_LENGTH) {
                write!(dst, "{name}: {value}{CRLF}").expect("writing to BytesMut can't fail");
            } else if !wrote_content_length {
                write!(dst, "{name}: {body_length}{CRLF}").expect("writing to BytesMut can't fail");
                wrote_content_length = true;
            }
        }
        if !wrote_content_length && body_length > 0 {
            write!(dst, "{CONTENT_LENGTH}: {body_length}{CRLF}")
                .expect("writing to BytesMut can't fail");
        }
    }
}

impl std::fmt::Display for MessageHeaders {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, (name, value)) in self.fields.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "{}: {}", name.escape_debug(), value.escape_debug())?;
        }

        Ok(())
    }
}

impl TryFrom<&[u8]> for MessageHeaders {
    type Error = MessageError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let lines = std::str::from_utf8(bytes).map_err(|_| {
            MessageError::InvalidEncoding(String::from_utf8_lossy(bytes).into_owned())
        })?;

        let fields = lines
            .split(CRLF)
            .filter(|line| !line.is_empty())
            .map(|line| {
                let (name, value) = line
                    .split_once(':')
                    .ok_or_else(|| MessageError::InvalidHeader(line.to_owned()))?;
                let name = name.trim();
                if name.is_empty() {
                    return Err(MessageError::InvalidHeader(line.to_owned()));
                }

                Ok((name.to_owned(), value.trim().to_owned()))
            })
            .collect::<Result<Vec<_>, MessageError>>()?;

        Ok(Self { fields })
    }
}

#[cfg(test)]
#[path = "test/message_headers_unittests.rs"]
mod tests;
