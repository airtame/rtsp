use crate::server::Server;

type ServerFuture = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;

pub struct ServerTask {
    future: ServerFuture,
}

impl ServerTask {
    pub(crate) fn new(server: Server) -> Self {
        Self { future: Box::pin(server.run_loop()) }
    }
}

impl std::future::Future for ServerTask {
    type Output = ();

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.future.as_mut().poll(cx)
    }
}

impl std::fmt::Debug for ServerTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ServerTask")
    }
}

#[cfg(test)]
#[path = "test/server_task_unittests.rs"]
mod tests;
