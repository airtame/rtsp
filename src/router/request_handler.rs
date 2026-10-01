use crate::message::{Request, Response};

pub trait RequestHandler: Send + Sync {
    fn handle(&self, request: &Request) -> Response;
}

// TODO(atokodi): Highlight in the rust docs that a handler can be either a type implementing
// RequestHandler or a plain function or closure taking &Request and returning a Response.
impl<F> RequestHandler for F
where
    F: Fn(&Request) -> Response + Send + Sync,
{
    fn handle(&self, request: &Request) -> Response {
        self(request)
    }
}
