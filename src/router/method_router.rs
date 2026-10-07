use crate::message::{MessageHeaderName, Request, RequestMethod, Response, StatusCode};
use crate::router::RequestHandler;

type Handlers = std::collections::HashMap<RequestMethod, std::sync::Arc<dyn RequestHandler>>;

#[derive(Clone, Default)]
pub struct MethodRouter {
    handlers: Handlers,
    fallback: Option<std::sync::Arc<dyn RequestHandler>>,
}

impl MethodRouter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_method(
        mut self,
        method: RequestMethod,
        handler: impl RequestHandler + 'static,
    ) -> Self {
        self.handlers.insert(method, std::sync::Arc::new(handler));
        self
    }

    pub fn with_fallback(mut self, handler: impl RequestHandler + 'static) -> Self {
        self.fallback = Some(std::sync::Arc::new(handler));
        self
    }

    fn methods(&self) -> Vec<String> {
        let mut methods: Vec<String> = self.handlers.keys().map(ToString::to_string).collect();
        methods.sort();
        methods
    }
}

impl std::fmt::Debug for MethodRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MethodRouter")
            .field("methods", &self.methods())
            .field("fallback", &self.fallback.is_some())
            .finish()
    }
}

impl RequestHandler for MethodRouter {
    fn handle(&self, request: &Request) -> Response {
        match self.handlers.get(request.method()).or(self.fallback.as_ref()) {
            Some(handler) => handler.handle(request),
            None => Response::new(request.version().clone(), StatusCode::MethodNotAllowed)
                .with_header(MessageHeaderName::Allow, self.methods().join(", ")),
        }
    }
}

#[cfg(test)]
#[path = "test/method_router_unittests.rs"]
mod tests;
