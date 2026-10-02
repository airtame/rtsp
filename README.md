# rtsp

Asynchronous RTSP 1.0/2.0 server and client connections, with request routing, built on
[Tokio](https://tokio.rs).

> **Status:** early development. The crate currently provides a TCP server that runs each
> accepted connection in its own task and shuts them all down gracefully when stopped.
> Incoming RTSP requests are answered by the `RequestHandler` registered for their path in the
> `Router` (`Server::with_router`), or with `404 Not Found`; responses from the peer are only
> logged. A message with an invalid start line is answered with `400 Bad Request` and the
> connection keeps going; prefix routing and the client side are not implemented. A connection
> stays open until the peer disconnects, the server stops, a message arrives whose end can't be
> determined (invalid headers or `Content-Length`, answered with `400 Bad Request` before
> closing), no complete message arrives within the optional idle timeout
> (`Server::with_connection_idle_timeout`), or the
> embedder closes it through the `ConnectionHandle` passed to
> `ServerDelegate::on_new_connection`.

## Building, running and testing

Requires Rust 1.85 or newer (edition 2024).

```sh
cargo build             # debug build
cargo build --release   # release build
cargo test              # unit tests
```

This crate is a library; to run it, use one of the [examples](#examples).

## Examples

### `server`

Starts a server with a 60 second connection idle timeout and two routes, showing both kinds of
handler:

- `/stream1`, a type implementing `RequestHandler`, answers `OPTIONS` with its supported
  methods, `DESCRIBE` with a small SDP description and any other method with
  `405 Method Not Allowed`.
- `/health`, a closure, answers every request with `200 OK`.

Every other path gets `404 Not Found`. It logs each
connection, every request and response and why the connection closed, and closes all open
connections on Ctrl+C.

```sh
cargo run --example server                 # listens on 127.0.0.1:8554
cargo run --example server 0.0.0.0:8554    # listens on a custom address
```

Log output defaults to `rtsp=debug`; override it with `RUST_LOG` (e.g. `RUST_LOG=rtsp=info`).
To try it, send a request from another terminal:

```sh
printf 'DESCRIBE rtsp://127.0.0.1:8554/stream1 RTSP/1.0\r\nCSeq: 1\r\n\r\n' | nc 127.0.0.1 8554
```

`nc` prints the `200 OK` response with the SDP body, the server logs the request and the
response, and the connection closes when `nc` exits. A connection
opened with plain `nc 127.0.0.1 8554` is closed after 60 seconds without a complete RTSP
message; typed lines don't count, since `nc` ends them with `\n` instead of `\r\n`.

## License

MIT, see [LICENSE](LICENSE).
