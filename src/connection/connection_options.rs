use crate::message::ParsingMode;

type ActivityHook = dyn Fn(std::net::SocketAddr) + Send + Sync;

#[derive(Clone, Default)]
pub struct ConnectionOptions {
    idle_timeout: Option<std::time::Duration>,
    parsing_mode: ParsingMode,
    activity_hook: Option<std::sync::Arc<ActivityHook>>,
}

impl ConnectionOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_idle_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.idle_timeout = Some(timeout);
        self
    }

    pub fn with_parsing_mode(mut self, parsing_mode: ParsingMode) -> Self {
        self.parsing_mode = parsing_mode;
        self
    }

    pub fn with_activity_hook(
        mut self,
        hook: impl Fn(std::net::SocketAddr) + Send + Sync + 'static,
    ) -> Self {
        self.activity_hook = Some(std::sync::Arc::new(hook));
        self
    }

    pub fn idle_timeout(&self) -> Option<std::time::Duration> {
        self.idle_timeout
    }

    pub fn parsing_mode(&self) -> ParsingMode {
        self.parsing_mode
    }

    pub fn activity_hook(&self) -> Option<&ActivityHook> {
        self.activity_hook.as_deref()
    }
}

impl std::fmt::Debug for ConnectionOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionOptions")
            .field("idle_timeout", &self.idle_timeout)
            .field("parsing_mode", &self.parsing_mode)
            .field("activity_hook", &self.activity_hook.is_some())
            .finish()
    }
}

#[cfg(test)]
#[path = "test/connection_options_unittests.rs"]
mod tests;
