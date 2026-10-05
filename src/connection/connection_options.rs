use crate::message::ParsingMode;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConnectionOptions {
    idle_timeout: Option<std::time::Duration>,
    parsing_mode: ParsingMode,
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

    pub fn idle_timeout(&self) -> Option<std::time::Duration> {
        self.idle_timeout
    }

    pub fn parsing_mode(&self) -> ParsingMode {
        self.parsing_mode
    }
}

#[cfg(test)]
#[path = "test/connection_options_unittests.rs"]
mod tests;
