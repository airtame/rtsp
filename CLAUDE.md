# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`rtsp` is a Rust library crate (edition 2024, Rust 1.85+) for asynchronous RTSP 1.0/2.0 connections on Tokio. It is in early development: the server and client work, and both sides can send requests through a `ConnectionHandle`, but prefix routing is not implemented. The **Status** paragraph in `README.md` describes connection behavior in detail. Feature commits have updated it along with the code, so keep it in sync when behavior changes. Known issues and deferred work are listed in `TODO.md`.

## Commands

CI (`.gitlab-ci.yml`) sets `RUSTFLAGS="-D warnings"`, so any warning fails the pipeline:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo test
```

Test paths follow `<module>::<file>::tests::<name>`:

```sh
cargo test run_answers_pipelined_requests_in_order   # single test by name
cargo test connection::connection::tests::            # one file's tests
cargo test message::                                  # everything under src/message
```

Example server: `cargo run --example server [addr]` listens on `127.0.0.1:8554` by default and logs at `rtsp=debug` (override with `RUST_LOG`). To send it a request:

```sh
printf 'DESCRIBE rtsp://127.0.0.1:8554/stream1 RTSP/1.0\r\nCSeq: 1\r\n\r\n' | nc 127.0.0.1 8554
```

Example client: `cargo run --example client [addr]` connects to `127.0.0.1:8554` by default and sends `OPTIONS` and `DESCRIBE` for `/stream1`, so run the server example first. To have a fake server send the client a request instead, start this before the client:

```sh
printf 'GET_PARAMETER rtsp://127.0.0.1:8554/ RTSP/1.0\r\nCSeq: 1\r\n\r\n' | nc -l 8554
```

## Architecture

`src/lib.rs` declares five private modules (`server`, `client`, `connection`, `message`, `router`) and re-exports the public API.

**Request flow.** `Server::run` accepts TCP connections and builds a `Connection` for each one, using the `ConnectionOptions` (idle timeout, `ParsingMode`) set with `Server::with_connection_options` (the defaults otherwise). The server passes the connection's `ConnectionHandle` to `ServerDelegate::on_new_connection`, then spawns `Connection::run` in a `JoinSet`. `Connection` wraps the stream in `Framed<TcpStream, MessageCodec>`. Each decoded request goes to the connection's `Arc<dyn RequestHandler>`. The server passes the handler set with `Server::with_handler`, an empty `Router` by default. `Connection` then copies the request's `CSeq` onto the response and writes it back. Copying `CSeq` in `Connection` means it applies to every handler, not just the router.

**Sending requests.** `ConnectionHandle::send` is synchronous. It puts a `PendingRequest` on an unbounded `mpsc` channel to the connection and returns a `ResponseFuture`. A `PendingRequest` is the request plus a `oneshot::Sender<Response>`, and the `ResponseFuture` wraps the matching `oneshot::Receiver`. Because the request is queued immediately, it goes out even if the future is only awaited later or dropped (fire-and-forget), and handlers and delegate callbacks can call `send` without blocking. A dedicated `select!` branch in `Connection::run` then:
- overwrites the caller's `CSeq` with the connection's own counter (`next_cseq`, starting at 1);
- writes the request;
- stores the oneshot in `pending_responses`, keyed by that `CSeq`.

An incoming response completes the oneshot whose `CSeq` it carries. Responses with an unknown or missing `CSeq` are logged and dropped. When `run` returns, the receiver and the pending oneshots are dropped, so pending and later `ResponseFuture`s resolve to `RequestError::ConnectionClosed`. If the connection is already gone, `send` drops the `PendingRequest` and its oneshot sender, so its future fails immediately without a special case. The peer's requests have their own `CSeq` sequence and don't interact with outgoing ones.

**Client.** `Client::connect` opens a TCP stream and builds the same `Connection`, using the client's handler (an empty `Router` by default), its `ConnectionOptions` and a fresh `CancellationToken`. It returns the `ConnectionHandle` and a `ConnectionTask`. `ConnectionTask` is a public boxed future around the crate-private `Connection::run`, and the embedder must await or spawn it. The server doesn't use `ConnectionTask`; it runs `Connection::run` directly in its `JoinSet`.

**Cancellation.** The server owns a `CancellationToken`. Each connection gets a `child_token()`, and the connection's `ConnectionHandle` holds that same child token. `Server::stop()` therefore closes every connection, while `ConnectionHandle::close()` closes only its own. After cancellation, `run` drains the `JoinSet`, so `on_connection_closed` fires for every connection before `run` returns. `Connection::run` returns a `ConnectionCloseReason` that says why the connection ended.

**Delegate callbacks run inline in the server's `select!` loop**, so they must not block.

**Idle timeout and activity hook.** The idle timer resets on every decoded frame, including malformed ones. Partial bytes don't reset it. The activity hook from `ConnectionOptions` is called with the peer address at the same point. It runs inline in the connection task, so it must not block.

**Two-tier error handling in `MessageCodec`.** `Decoder::Item` is `Result<Message, MalformedMessage>` and `Decoder::Error` is `MessageError`:
- **Bad start line, framing still known.** The decoder yields `Ok(Some(Err(MalformedMessage)))`. The connection replies `400` (echoing `CSeq` if it is numeric) and keeps reading.
- **Framing lost** (bad header line, non-UTF-8 headers or an invalid `Content-Length`). The decoder returns `Err(MessageError)`. The connection replies `400`, then closes with `InvalidMessage`.
- **`MessageError::Io`.** The connection closes with `Io` and sends no reply.

**Parsing mode.** `MessageCodec` holds the connection's `ParsingMode` and passes it through `Message::new` to `Request::parse` and `Response::parse`. In `Strict` mode (the default), a `Version::Other` (such as `HTTP/1.1`) is rejected as an invalid start line, and a request without `CSeq` fails with `MessageError::MissingHeader`. Both are malformed-message errors, so the peer gets a `400` and the connection stays open. `Lenient` mode accepts both. `Version::from_str` doesn't depend on the mode, so unknown RTSP versions such as `RTSP/1.1` are rejected in both modes.

Known gaps in the codec (no limit on header size or `Content-Length`, and EOF in the middle of a message) are tracked as TODOs in `message_codec.rs`.

**Request vs. response.** `Message::new` checks whether the first token of the start line contains `/`. If it does (as in `RTSP/1.0 200 OK`), the message is parsed as a response; otherwise it is parsed as a request.

**Router.** `Router` wraps `Arc<RwLock<HashMap<String, Arc<dyn RequestHandler>>>>`, so its clones share one route table. Each connection gets a clone, and `register`/`unregister` calls made after the server starts apply to live connections. Matching is exact on `Request::path()`, which has the scheme, authority, query and fragment removed. A request with no matching route goes to the fallback set with `with_fallback`, or gets `404`. The fallback is a plain field outside the shared table, so it is set when the router is built and copied into later clones. `Router` implements `RequestHandler` by calling `route()`, so a router can be used anywhere a handler is expected. `RequestHandler` is synchronous and has a blanket impl for `Fn(&Request) -> Response + Send + Sync` closures.

**MethodRouter.** `MethodRouter` maps a `RequestMethod` to a handler. It is built with `with_method` and `with_fallback` and doesn't change afterwards. A method with no handler goes to the fallback, or gets `405` with an `Allow` header listing the registered methods in sorted order. It ignores the path, so used as a `Router` fallback it handles a method on any path (such as `SETUP` on a per-session URI), and registered for a path it limits which methods that path accepts. `"*"` is not a wildcard route: it is the path of `OPTIONS * RTSP/1.0`.

**Encoding.** `MessageHeaders::encode` always writes `Content-Length` from the actual body length. It overwrites any value the handler set, and it adds the header when the body is non-empty.

## Conventions

- One type per file, with the file named after the type in snake_case. `mod.rs` files only declare submodules and re-export. Crate-internal items are `pub(crate)`, and the `unreachable_pub` lint is on.
- External types are written with fully qualified paths (`std::net::SocketAddr`, `tokio_util::sync::CancellationToken`, `tokio_util::bytes::Bytes`) instead of being imported. `use` is reserved for `crate::` items and extension traits (`SinkExt`, `StreamExt`, `std::fmt::Write as _`).
- Optional configuration uses builder-style `with_*(mut self, ..) -> Self` methods.
- Log messages start with `[rtsp]`.
- Protocol enums (`Version`, `RequestMethod`, `StatusCode`) are `#[non_exhaustive]` and have a catch-all variant (`Other(String)`, `Extension(String)`, `Extension(u16, String)` for a code and its reason phrase). A `Response` holds one `StatusCode`, which carries both the code and the reason phrase. `Response::new` panics if the reason phrase contains a line break. `Response::parse` turns a known code into its named variant and drops the peer's reason phrase, and turns an unknown code into `Extension(code, reason phrase)`.
- `rustfmt.toml` sets `use_small_heuristics = "Max"`.
- There are no rustdoc comments yet. Open questions are left as `// TODO(atokodi): ...` comments.

## Tests

Unit tests live in a `test/` directory next to the module, in a file named `<file>_unittests.rs`. They are included from the bottom of the source file, which lets them reach private and `pub(crate)` items through `use super::*;`:

```rust
#[cfg(test)]
#[path = "test/connection_unittests.rs"]
mod tests;
```

The parser test helpers (`request()`, `response()`, `message()`, and the router's `request()`) parse in `Lenient` mode, so start lines can be tested without a `CSeq` header. Strict-mode rules need their own tests.

Connection and server tests run `#[tokio::test]` over real TCP sockets bound to `127.0.0.1:0` and wrap their waits in `tokio::time::timeout`. Test names are descriptive snake_case sentences, such as `run_answers_malformed_start_line_with_bad_request_and_stays_open`.

## Git

Branches are named `noissue-<kebab-description>` and commits are titled `[Noissue] <Imperative summary>`. Changes reach `main` through GitLab merge requests.
