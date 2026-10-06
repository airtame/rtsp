use crate::connection::RequestError;
use crate::message::Response;

pub struct ResponseFuture {
    response_rx: tokio::sync::oneshot::Receiver<Response>,
}

impl ResponseFuture {
    pub(crate) fn new(response_rx: tokio::sync::oneshot::Receiver<Response>) -> Self {
        Self { response_rx }
    }
}

impl std::future::Future for ResponseFuture {
    type Output = Result<Response, RequestError>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::pin::Pin::new(&mut self.response_rx)
            .poll(cx)
            .map(|result| result.map_err(|_| RequestError::ConnectionClosed))
    }
}

impl std::fmt::Debug for ResponseFuture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResponseFuture")
    }
}

#[cfg(test)]
#[path = "test/response_future_unittests.rs"]
mod tests;
