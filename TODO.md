# TODO

## Connection

- Race every write in `Connection::run` against the cancellation token. While a write is blocked
  (the peer stops reading and the send buffer fills), the loop doesn't poll cancellation or the
  idle timer, so `ConnectionHandle::close()` and `ServerHandle::stop()` can't end the connection, and
  every caller awaiting a `ResponseFuture` is stuck too. Later, consider splitting the `Framed`
  stream into read and write halves so a slow write doesn't stop reading either.
- Clean up abandoned entries in the pending-request map. A request whose `ResponseFuture` was
  dropped or timed out keeps its entry until the response arrives or the connection closes.
  Sweep entries whose `oneshot` is closed, or give each request a deadline inside the
  connection.
- The request queue between `ConnectionHandle` and `Connection` is unbounded, so `send` can
  queue a request without awaiting. If a cap is ever needed, switch to a bounded channel with
  `try_send` and add a `RequestError::QueueFull`.
- `ConnectionHandle::is_closed()` only reports `close()` and `ServerHandle::stop()`. Also report a
  connection that ended for any other reason, e.g. by checking whether the request channel is
  closed.

## Client

- Keep `Connection` at the transport level (request channel, `CSeq`, matching responses).
  Build `Session` handling, authentication (401 → retry with `Authorization`), redirects and
  request ordering (`SETUP` before `PLAY`) in a layer above it.

## Codec

- Support interleaved binary data (`$` framing, RFC 2326 §10.12). Once a client sends `PLAY`
  with `Transport: RTP/AVP/TCP;interleaved=...`, the server mixes `$` frames into the RTSP
  stream, and `MessageCodec` can't parse them.
- Limit the size of a message head. `MessageCodec::decode` buffers bytes until it finds the blank
  line that ends the headers (`\r\n\r\n`), so a peer that never sends one makes it read
  indefinitely.
- Limit `Content-Length`. It has no upper limit, so a peer can make `MessageCodec::decode`
  reserve whatever size it claims, and a value near `usize::MAX` overflows the message length.
- Override `MessageCodec::decode_eof`. A peer that disconnects in the middle of a message makes
  the default `decode_eof` return an I/O error ("bytes remaining on stream"), so the connection
  closes as `Io` instead of `ClosedByPeer`.
- Find a better way to split the start line from the headers than `split_start_line` in
  `message_codec.rs`.
- Decide what to do with a message that repeats a single-value header (two `CSeq` or `Session`
  lines, say) and with its connection. `MessageCodec::decode` only logs a warning with the
  message and passes it on, so the handler sees the first value. It could instead answer
  `400 Bad Request` and keep the connection open, or close the connection as it does when
  `Content-Length` repeats.

## Messages

- Always write `Content-Length` in `Response::encode` (0 for an empty body), except for 1xx, 204
  and 304 responses. Without it an HTTP client in lenient mode reads the body until the
  connection closes, so it waits for the idle timeout.
- `MessageHeaders::append` panics on an invalid header name or value. Should it return a
  `Result` instead?
- Rename `MessageError` to `HeaderError` and add a separate `MessageError`? Or use
  `std::io::Error` with `ErrorKind::InvalidData` instead, as `version.rs` does?

## Documentation

- Document (in rustdoc) that request handlers and `ServerDelegate` callbacks must not block,
  e.g. with `block_on` on a `ResponseFuture`. A handler runs on its connection's task, so
  blocking on a response from the same connection deadlocks, and a delegate callback blocks the
  whole server loop. Async handlers would remove the reason to block, but only fix the deadlock
  if the connection runs handler futures alongside its loop and keeps responses in request
  order.
- Highlight in the rustdoc that a handler can be either a type implementing `RequestHandler` or a
  plain function or closure taking `&Request` and returning a `Response`.
