use crate::message::{
    CSeqHeader, MalformedMessage, Message, MessageError, MessageHeaderName, MessageHeaders,
    ParsingMode,
};

const CRLF: &[u8] = b"\r\n";
const DOUBLE_CRLF: &[u8] = b"\r\n\r\n";

#[derive(Debug, Default)]
pub(crate) struct MessageCodec {
    parsing_mode: ParsingMode,
}

impl MessageCodec {
    pub(crate) fn new(parsing_mode: ParsingMode) -> Self {
        Self { parsing_mode }
    }
}

impl tokio_util::codec::Decoder for MessageCodec {
    type Item = Result<Message, MalformedMessage>;
    type Error = MessageError;

    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let Some(message_head_end) =
            src.windows(DOUBLE_CRLF.len()).position(|window| window == DOUBLE_CRLF)
        else {
            return Ok(None);
        };

        let (start_line, header_lines) = split_start_line(&src[..message_head_end]);
        let start_line_length = start_line.len();
        let headers = MessageHeaders::try_from(header_lines)?;

        let body_length = content_length(&headers)?;

        let message_length = message_head_end + DOUBLE_CRLF.len() + body_length;
        if src.len() < message_length {
            src.reserve(message_length - src.len());
            return Ok(None);
        }

        let raw_message = src.split_to(message_length).freeze();
        let start_line = &raw_message[..start_line_length];
        let body = raw_message.slice(message_head_end + DOUBLE_CRLF.len()..);
        let cseq = headers.typed::<CSeqHeader>().and_then(Result::ok);
        let has_repeated_single_value_header = {
            let mut seen_names = std::collections::HashSet::new();
            headers.iter().any(|(name, _)| !name.allows_multiple() && !seen_names.insert(name))
        };

        let message = Message::new(start_line, headers, body, self.parsing_mode);
        if let Ok(message) = &message {
            if has_repeated_single_value_header {
                log::warn!("[rtsp] message repeats a single-value header:\n{message}");
            }
        }

        Ok(Some(message.map_err(|error| MalformedMessage { error, cseq })))
    }
}

fn content_length(headers: &MessageHeaders) -> Result<usize, MessageError> {
    let values = headers.get_all(MessageHeaderName::ContentLength).collect::<Vec<_>>();

    match values.as_slice() {
        [] => Ok(0),
        [value] => value.parse().map_err(|_| MessageError::InvalidContentLength(value.to_string())),
        values => Err(MessageError::InvalidContentLength(values.join(", "))),
    }
}

fn split_start_line(message_head: &[u8]) -> (&[u8], &[u8]) {
    match message_head.windows(CRLF.len()).position(|window| window == CRLF) {
        Some(start_line_end) => {
            (&message_head[..start_line_end], &message_head[start_line_end + CRLF.len()..])
        }
        None => (message_head, &[]),
    }
}

impl tokio_util::codec::Encoder<Message> for MessageCodec {
    type Error = std::io::Error;

    fn encode(
        &mut self,
        message: Message,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        message.encode(dst);
        Ok(())
    }
}

#[cfg(test)]
#[path = "test/message_codec_unittests.rs"]
mod tests;
