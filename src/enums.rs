#![allow(dead_code)]

build_integer_enum!(CompressionScheme, u64,
    NoCompression, 1,
    LZW,           5,
    AdobeDeflate,  8,
    PackBits,      32773,
    Deflate,       32946);
