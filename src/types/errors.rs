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
    AddressOverflowError,
    IntOverflowError,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LibError {
    UnexpectedLibError,
    ObjectParseFailure(ParseError),
    ParseLibError,
    IOError,
}

impl From<std::io::Error> for LibError {
    fn from(_: std::io::Error) -> Self {
        LibError::IOError
    }
}
