use crate::message::{Message, MessageError, MessageHeaders};

const CRLF: &[u8] = b"\r\n";
const DOUBLE_CRLF: &[u8] = b"\r\n\r\n";

#[derive(Debug, Default)]
pub(crate) struct MessageCodec {}

// TODO(atokodi): A peer that disconnects in the middle of a message makes the default decode_eof
// return an I/O error ("bytes remaining on stream"), so the connection closes as Io instead of
// ClosedByPeer. Override decode_eof to handle that case.
impl tokio_util::codec::Decoder for MessageCodec {
    type Item = Message;
    type Error = MessageError;

    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        // TODO(atokodi): This will read indefinitely if DOUBLE_CRLF is never read.
        let Some(message_head_end) =
            src.windows(DOUBLE_CRLF.len()).position(|window| window == DOUBLE_CRLF)
        else {
            return Ok(None);
        };

        let (start_line, header_lines) = split_start_line(&src[..message_head_end]);
        let start_line_length = start_line.len();
        let headers = MessageHeaders::try_from(header_lines)?;

        // TODO(atokodi): Content-Length has no upper limit, so a peer can make reserve() below
        // allocate whatever size it claims, and a value near usize::MAX overflows message_length.
        let body_length: usize = match headers.get("Content-Length") {
            Some(value) => {
                value.parse().map_err(|_| MessageError::InvalidContentLength(value.to_owned()))?
            }
            None => 0,
        };

        let message_length = message_head_end + DOUBLE_CRLF.len() + body_length;
        if src.len() < message_length {
            src.reserve(message_length - src.len());
            return Ok(None);
        }

        let raw_message = src.split_to(message_length).freeze();
        let start_line = &raw_message[..start_line_length];
        let body = raw_message.slice(message_head_end + DOUBLE_CRLF.len()..);

        Ok(Some(Message::new(start_line, headers, body)?))
    }
}

// TODO(atokodi): I don't really like this but not sure how to do it better for now.
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
