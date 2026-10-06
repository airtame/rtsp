# rtsp

Asynchronous RTSP 1.0/2.0 server and client connections, with request routing, built on
[Tokio](https://tokio.rs).

> **Status:** early development. The crate currently provides a TCP server that runs each
> accepted connection in its own task and shuts them all down gracefully when stopped, and a
> `Client` whose `connect` returns a `ConnectionHandle` and a `ConnectionTask`, a future that
> drives the connection until it closes. Incoming RTSP requests are answered by the
> `RequestHandler` set with `Server::with_handler` or `Client::with_handler` (an empty `Router`
> by default). A `Router` passes each request to the handler registered for its path, to its
> fallback (`Router::with_fallback`) if no path matches, or answers `404 Not Found`. A
> `MethodRouter` does the same by request method and answers `405 Method Not Allowed` with an
> `Allow` header. Both are handlers themselves, so a `MethodRouter` can be registered for a path
> or used as a `Router`'s fallback to handle a method such as `SETUP` on any path. The response
> gets the request's `CSeq`. Either side can send its own requests with `ConnectionHandle::send`, which queues the
> request right away and returns a `ResponseFuture` that resolves to the matching response;
> dropping the future doesn't cancel the request. The connection numbers outgoing requests with
> its own `CSeq` and matches responses by it. A response that matches no pending request is
> logged and dropped, and the future resolves to `RequestError::ConnectionClosed` if the
> connection closes first. A
> message with an invalid start line is answered with `400 Bad Request` and the connection
> keeps going; prefix routing is not implemented. A connection stays open until the peer
> disconnects, the server stops, a message arrives whose end can't be determined (invalid
> headers or `Content-Length`, answered with `400 Bad Request` before closing), no complete
> message arrives within the optional idle timeout (`ConnectionOptions::with_idle_timeout`,
> set with `Server::with_connection_options` or `Client::with_connection_options`), or the embedder closes it through its `ConnectionHandle`
> (passed to `ServerDelegate::on_new_connection`, or returned by `Client::connect`). `ConnectionOptions` also
> sets the `ParsingMode`. In `Strict` mode (the default) messages must use RTSP/1.0 or RTSP/2.0
> and requests must carry `CSeq`; anything else is answered with `400 Bad Request` and the
> connection keeps going. `Lenient` mode also accepts other versions such as `HTTP/1.1`, and
> requests without `CSeq`. An optional activity hook (`ConnectionOptions::with_activity_hook`)
> is called with the peer address each time a complete message arrives.

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

Starts a server with a 60 second connection idle timeout and a `Router` that shows the ways to
combine handlers:

- `/stream1` is a `MethodRouter`. A function answers `OPTIONS` with the supported methods, a
  type implementing `RequestHandler` answers `DESCRIBE` with a small SDP description, and any
  other method gets `405 Method Not Allowed` with an `Allow` header.
- `/health`, a closure, answers every request with `200 OK`, whatever its method.
- The router's fallback is another `MethodRouter`. It answers `OPTIONS` on every other path,
  including `OPTIONS *`, and its own fallback answers anything else with `404 Not Found`.

It logs each
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

### `client`

Connects to a server, spawns the `ConnectionTask` returned by `Client::connect`, sends `OPTIONS`
and `DESCRIBE` for `/stream1` through the `ConnectionHandle` and prints the responses. It
answers every request the server sends with `200 OK`, prints a line for each message it
receives, disconnects on Ctrl+C and prints why the connection closed.

```sh
cargo run --example client                 # connects to 127.0.0.1:8554
cargo run --example client 10.0.0.5:8554   # connects to a custom address
```

To try it, start the `server` example in another terminal first. The client prints the
`OPTIONS` response with the `Public` methods and the `DESCRIBE` response with the SDP body, and
pressing Ctrl+C in the client closes the connection ("connection cancelled" in the client,
"connection closed by peer" in the server). The `server` example never sends requests, so to see
the client's handler answer one, play the server with `nc` instead:

```sh
printf 'GET_PARAMETER rtsp://127.0.0.1:8554/ RTSP/1.0\r\nCSeq: 1\r\n\r\n' | nc -l 8554
```

Then start the client. It prints the received message and answers it, and `nc` prints the
`200 OK` response with `CSeq: 1`. `nc` then closes the connection without answering the
client's `OPTIONS`, so the client reports that `OPTIONS` failed and that the peer closed the
connection.

## License

MIT, see [LICENSE](LICENSE).
