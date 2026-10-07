use crate::connection::PendingRequest;
use crate::message::{MalformedMessage, MessageError, Request, Response};

#[derive(Debug)]
pub(crate) enum ConnectionEvent {
    Cancelled,
    IdleTimeout,
    ClosedByPeer,
    ReadFailed(std::io::Error),
    InvalidMessage(MessageError),
    MalformedMessage(MalformedMessage),
    RequestReceived(Request),
    ResponseReceived(Response),
    RequestQueued(PendingRequest),
}
