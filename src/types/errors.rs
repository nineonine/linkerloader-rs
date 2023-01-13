
#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    UnexpectedParseError,
    MissingMagicNumber,
    InvalidMagicNumber,
    MissingNSegsNSumsNRels,
    InvalidNSegsNSumsNRels,
    InvalidNSegsValue,
    InvalidNSymsValue,
    InvalidNRelsValue,
}
