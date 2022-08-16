#![allow(dead_code)]

use std::fs::File;
use std::io::{BufReader, Read};
use crate::TiffError;


#[derive(Debug, Default)]
pub struct Header {
    pub byte_order: u16,
    pub version:    u16,
    pub first_ifd:  u64,
}
impl Header {
    pub fn from_buffer(buffer: &mut BufReader<File>) -> Result<Self, TiffError> {
        let byte_order = buffer_as!(buffer, u16, true)?;
        let le = match byte_order {
            0x4949 => true,
            0x4d4d => false,
            _ => return Err(TiffError::UnexpectedHeaderByteOrder),
        };

        let version = buffer_as!(buffer, u16, le)?;
        let first_ifd: u64;
        match version {
            42 => first_ifd = buffer_as!(buffer, u32, le)? as u64,
            43 => {
                let offset_size = buffer_as!(buffer, u16, le)?;
                if offset_size != 8 {
                    return Err(TiffError::UnexpectedHeaderOffsetSize);
                }
                let reserved = buffer_as!(buffer, u16, le)?;
                if reserved != 0 {
                    return Err(TiffError::UnexpectedHeaderReserved);
                }
                first_ifd = buffer_as!(buffer, u64, le)?;
            },
            _ => return Err(TiffError::UnexpectedHeaderVersion),
        }

        Ok(Self{byte_order, version, first_ifd})
    }
    pub fn is_btf(&self) -> bool {
        self.version == 43
    }
    pub fn is_le(&self) -> bool {
        self.byte_order == 0x4949
    }
}
