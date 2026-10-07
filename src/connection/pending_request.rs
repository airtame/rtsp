use crate::message::{Request, Response};

#[derive(Debug)]
pub(crate) struct PendingRequest {
    pub(crate) request: Request,
    pub(crate) response_tx: tokio::sync::oneshot::Sender<Response>,
}
