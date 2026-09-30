# rtsp

Asynchronous RTSP 1.0/2.0 server and client connections, with request routing, built on
[Tokio](https://tokio.rs).

> **Status:** early development. The crate currently provides a TCP server that accepts
> connections and can be shut down gracefully. RTSP message parsing, request routing and
> client connections are not implemented yet; accepted connections are logged and closed.

## Building, running and testing

Requires Rust 1.85 or newer (edition 2024).

```sh
cargo build             # debug build
cargo build --release   # release build
```

This crate is a library; to run it, use one of the [examples](#examples).

## Examples

### `server`

Starts a server, logs incoming connections and shuts down on Ctrl+C.

```sh
cargo run --example server                 # listens on 127.0.0.1:8554
cargo run --example server 0.0.0.0:8554    # listens on a custom address
```

Log output defaults to `rtsp=debug`; override it with `RUST_LOG` (e.g. `RUST_LOG=rtsp=info`).
To try it, connect from another terminal with `nc -z 127.0.0.1 8554`:

```text
[2026-09-30T09:51:59Z DEBUG rtsp::server::server] [rtsp] server bind successful to 127.0.0.1:8554
RTSP server listening on rtsp://127.0.0.1:8554, press Ctrl+C to stop
[2026-09-30T09:51:59Z DEBUG rtsp::server::server] [rtsp] server run loop started
[2026-09-30T09:51:59Z DEBUG rtsp::server::server] [rtsp] new connection from 127.0.0.1:61959
^C[2026-09-30T09:51:59Z DEBUG rtsp::server::server] [rtsp] server stop called
[2026-09-30T09:51:59Z DEBUG rtsp::server::server] [rtsp] server cancellation token triggered
RTSP server stopped
```

## License

MIT, see [LICENSE](LICENSE).
