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
- Document (in rustdoc) that request handlers and `ServerDelegate` callbacks must not block,
  e.g. with `block_on` on a `ResponseFuture`. A handler runs on its connection's task, so
  blocking on a response from the same connection deadlocks, and a delegate callback blocks the
  whole server loop. Async handlers would remove the reason to block, but only fix the deadlock
  if the connection runs handler futures alongside its loop and keeps responses in request
  order.
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
