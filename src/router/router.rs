use crate::message::{Request, Response, StatusCode};
use crate::router::RequestHandler;

const CSEQ: &str = "CSeq";

type Routes = std::collections::HashMap<String, std::sync::Arc<dyn RequestHandler>>;

#[derive(Clone, Default)]
pub struct Router {
    routes: std::sync::Arc<std::sync::RwLock<Routes>>,
}

impl Router {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &self,
        path: impl Into<String>,
        handler: impl RequestHandler + 'static,
    ) -> Option<std::sync::Arc<dyn RequestHandler>> {
        self.write().insert(path.into(), std::sync::Arc::new(handler))
    }

    pub fn unregister(&self, path: &str) -> Option<std::sync::Arc<dyn RequestHandler>> {
        self.write().remove(path)
    }

    pub fn get(&self, path: &str) -> Option<std::sync::Arc<dyn RequestHandler>> {
        self.read().get(path).cloned()
    }

    pub(crate) fn route(&self, request: &Request) -> Response {
        let response = match self.get(request.path()) {
            Some(handler) => handler.handle(request),
            None => Response::new(request.version().clone(), StatusCode::NotFound),
        };

        match request.headers().get(CSEQ) {
            Some(cseq) => response.with_cseq(cseq),
            None => response,
        }
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, Routes> {
        self.routes.read().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, Routes> {
        self.routes.write().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl std::fmt::Debug for Router {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let routes = self.read();
        let mut paths: Vec<&String> = routes.keys().collect();
        paths.sort();

        f.debug_struct("Router").field("paths", &paths).finish()
    }
}

#[cfg(test)]
#[path = "test/router_unittests.rs"]
mod tests;
