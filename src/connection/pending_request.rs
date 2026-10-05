use crate::message::{Request, Response};

pub(crate) struct PendingRequest {
    pub(crate) request: Request,
    pub(crate) response_tx: tokio::sync::oneshot::Sender<Response>,
}
