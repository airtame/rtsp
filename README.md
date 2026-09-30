# rtsp

Asynchronous RTSP 1.0/2.0 server and client connections, with request routing, built on
[Tokio](https://tokio.rs).

> **Status:** early development. The crate currently provides a TCP server that runs each
> accepted connection in its own task and shuts them all down gracefully when stopped. RTSP
> message parsing, request routing and the client side are not implemented yet; data received
> on a connection is discarded, and connections stay open until the peer disconnects, the
> server stops, an optional idle timeout (`Server::with_connection_idle_timeout`) expires, or
> the embedder closes it through the `ConnectionHandle` passed to
> `ServerDelegate::on_new_connection`.

## Building, running and testing

Requires Rust 1.85 or newer (edition 2024).

```sh
cargo build             # debug build
cargo build --release   # release build
```

This crate is a library; to run it, use one of the [examples](#examples).

## Examples

### `server`

Starts a server with a 60 second connection idle timeout, logs each connection and why it
closed, and closes all open connections on Ctrl+C.

```sh
cargo run --example server                 # listens on 127.0.0.1:8554
cargo run --example server 0.0.0.0:8554    # listens on a custom address
```

Log output defaults to `rtsp=debug`; override it with `RUST_LOG` (e.g. `RUST_LOG=rtsp=info`).
To try it, open connections from another terminal with `nc 127.0.0.1 8554`. A connection
closes when `nc` exits, when nothing is typed into `nc` for 60 seconds, or when the server is
stopped.

## License

MIT, see [LICENSE](LICENSE).
