use crate::message::{Request, Response};

pub trait RequestHandler: Send + Sync {
    fn handle(&self, request: &Request) -> Response;
}

impl<F> RequestHandler for F
where
    F: Fn(&Request) -> Response + Send + Sync,
{
    fn handle(&self, request: &Request) -> Response {
        self(request)
    }
}
