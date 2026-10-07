use crate::message::{MessageError, MessageHeader, MessageHeaderName};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CSeqHeader(pub u32);

impl MessageHeader for CSeqHeader {
    const NAME: MessageHeaderName = MessageHeaderName::CSeq;

    fn decode(value: &str) -> Result<Self, MessageError> {
        value
            .parse()
            .map(Self)
            .map_err(|_| MessageError::InvalidHeaderValue(Self::NAME, value.to_owned()))
    }

    fn encode(&self) -> String {
        self.0.to_string()
    }
}

#[cfg(test)]
#[path = "test/cseq_header_unittests.rs"]
mod tests;
