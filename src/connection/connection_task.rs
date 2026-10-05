use crate::connection::{Connection, ConnectionCloseReason};

type ConnectionFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = ConnectionCloseReason> + Send>>;

pub struct ConnectionTask {
    future: ConnectionFuture,
}

impl ConnectionTask {
    pub(crate) fn new(connection: Connection) -> Self {
        Self { future: Box::pin(connection.run()) }
    }
}

impl std::future::Future for ConnectionTask {
    type Output = ConnectionCloseReason;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.future.as_mut().poll(cx)
    }
}

impl std::fmt::Debug for ConnectionTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ConnectionTask")
    }
}

#[cfg(test)]
#[path = "test/connection_task_unittests.rs"]
mod tests;
