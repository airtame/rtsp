use super::*;

#[test]
fn default_is_strict() {
    assert_eq!(ParsingMode::default(), ParsingMode::Strict);
}

#[test]
fn debug_strict() {
    assert_eq!(format!("{:?}", ParsingMode::Strict), "Strict");
}

#[test]
fn debug_lenient() {
    assert_eq!(format!("{:?}", ParsingMode::Lenient), "Lenient");
}
