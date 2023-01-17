
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

    InvalidSegment,
    InvalidSegmentName,
    InvalidSegmentStart,
    InvalidSegmentLen,
    InvalidSegmentDescr,
    InvalidNumOfSegments,

    InvalidSymbolTableEntry,
    InvalidSTEType,
    InvalidSTEValue,
    InvalidSTESegment,
    InvalidNumOfSTEs,
    STESegmentRefOutOfRange,
    NonZeroSegmentForUndefinedSTE,

    InvalidRelocationEntry,
    InvalidRelRef,
    RelSegmentOutOfRange,
    RelSymbolOutOfRange,
    InvalidRelType,
    InvalidRelSegment,
    InvalidNumOfRelocations,
}
