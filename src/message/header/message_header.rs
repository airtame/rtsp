use crate::message::{MessageError, MessageHeaderName};

pub trait MessageHeader: Sized {
    const NAME: MessageHeaderName;

    fn decode(value: &str) -> Result<Self, MessageError>;
    fn encode(&self) -> String;
}
