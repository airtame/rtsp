use crate::message::MessageError;

#[derive(Debug)]
pub(crate) struct MalformedMessage {
    pub(crate) error: MessageError,
    pub(crate) cseq: Option<String>,
}
