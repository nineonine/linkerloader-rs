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

    InvalidRelocationEntry,
    InvalidRelRef,
    RelSegmentOutOfRange,
    RelSymbolOutOfRange,
    InvalidRelType,
    InvalidRelSegment,
    InvalidNumOfRelocations,

    InvalidObjectData,
    SegmentDataLengthMismatch,
    SegmentDataOutOfBounds,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LinkError {
    UnexpectedLinkError,
    DuplicateObjectError,
    MultipleSymbolDefinitions,
    UndefinedSymbolError,
}
