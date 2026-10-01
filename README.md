# rtsp

Asynchronous RTSP 1.0/2.0 server and client connections, with request routing, built on
[Tokio](https://tokio.rs).

> **Status:** early development. The crate currently provides a TCP server that runs each
> accepted connection in its own task and shuts them all down gracefully when stopped.
> Incoming RTSP requests and responses are parsed and logged, but not answered yet; request
> routing, sending responses and the client side are not implemented. A connection stays open
> until the peer disconnects, the server stops, an invalid message arrives, no complete message
> arrives within the optional idle timeout (`Server::with_connection_idle_timeout`), or the
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

Starts a server with a 60 second connection idle timeout, logs each connection, every message
it receives and why the connection closed, and closes all open connections on Ctrl+C.

```sh
cargo run --example server                 # listens on 127.0.0.1:8554
cargo run --example server 0.0.0.0:8554    # listens on a custom address
```

Log output defaults to `rtsp=debug`; override it with `RUST_LOG` (e.g. `RUST_LOG=rtsp=info`).
To try it, send a request from another terminal:

```sh
printf 'OPTIONS * RTSP/1.0\r\nCSeq: 1\r\n\r\n' | nc 127.0.0.1 8554
```

The server logs the parsed request, and the connection closes when `nc` exits. A connection
opened with plain `nc 127.0.0.1 8554` is closed after 60 seconds without a complete RTSP
message; typed lines don't count, since `nc` ends them with `\n` instead of `\r\n`.

## License

MIT, see [LICENSE](LICENSE).
