use crate::message::{CSeqHeader, MessageError};

#[derive(Debug)]
pub(crate) struct MalformedMessage {
    pub(crate) error: MessageError,
    pub(crate) cseq: Option<CSeqHeader>,
}
