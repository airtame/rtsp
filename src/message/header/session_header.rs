use crate::message::{MessageError, MessageHeader, MessageHeaderName};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionHeader {
    pub id: String,
    pub timeout: Option<std::time::Duration>,
}

impl MessageHeader for SessionHeader {
    const NAME: MessageHeaderName = MessageHeaderName::Session;

    fn decode(value: &str) -> Result<Self, MessageError> {
        let invalid = || MessageError::InvalidHeaderValue(Self::NAME, value.to_owned());
        let mut parts = value.split(';');
        let id = parts.next().map(str::trim).filter(|id| !id.is_empty()).ok_or_else(invalid)?;

        let mut timeout = None;
        for parameter in parts {
            let (name, seconds) = parameter.split_once('=').ok_or_else(invalid)?;
            if name.trim().eq_ignore_ascii_case("timeout") {
                let seconds = seconds.trim().parse().map_err(|_| invalid())?;
                timeout = Some(std::time::Duration::from_secs(seconds));
            }
        }

        Ok(Self { id: id.to_owned(), timeout })
    }

    fn encode(&self) -> String {
        match self.timeout {
            Some(timeout) => format!("{};timeout={}", self.id, timeout.as_secs()),
            None => self.id.clone(),
        }
    }
}

#[cfg(test)]
#[path = "test/session_header_unittests.rs"]
mod tests;
