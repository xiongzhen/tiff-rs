
macro_rules! buffer_as {
    ($buffer:expr, $typ:ty, $le:expr) => {
        {
            let mut data = [0u8; std::mem::size_of::<$typ>()];
            if let Ok(n) = $buffer.read(&mut data) {
                if n != std::mem::size_of::<$typ>() {
                    Err(TiffError::UnexpectedEndOfBuffer)
                } else {
                    match $le {
                        true  => Ok(<$typ>::from_le_bytes(data)),
                        false => Ok(<$typ>::from_be_bytes(data)),
                    }
                }
            } else {
                Err(TiffError::UnexpectedEndOfBuffer)
            }
        }
    }
}
macro_rules! buffer_as_offset {
    ($buffer:expr, $btf:expr, $le:expr) => {
        if $btf {
            buffer_as!($buffer, u64, $le)?
        } else {
            buffer_as!($buffer, u32, $le)? as u64
        }
    }
}
