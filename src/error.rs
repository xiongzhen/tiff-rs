#![allow(dead_code)]

#[derive(Debug)]
pub enum TiffError {
    UnexpectedEndOfBuffer,
    UnexpectedHeaderByteOrder,
    UnexpectedHeaderVersion,
    UnexpectedHeaderOffsetSize,
    UnexpectedHeaderReserved,
    CannotOpenFile,
    UnknownBufferError,
    UnknownTagKind,
    UnknownTagId,
    InvalidIndex,
    IncompatibleTagDataKind,
    CannotFindTag,
    NotSupportedCompressionScheme,
}
