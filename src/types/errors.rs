
#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    ParseError(String),
    MissingMagicNumber,
    InvalidMagicNumber,
}
