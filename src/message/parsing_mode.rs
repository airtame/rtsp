#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ParsingMode {
    #[default]
    Strict,
    Lenient,
}

#[cfg(test)]
#[path = "test/parsing_mode_unittests.rs"]
mod tests;
