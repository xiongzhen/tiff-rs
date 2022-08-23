#![allow(dead_code)]

use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom, Read};
use crate::{TiffError, CompressionScheme};

fn array_from_slice<const N: usize>(slice: &[u8]) -> &[u8; N] {
    <&[u8] as std::convert::TryInto<&[u8; N]>>::try_into(slice).unwrap()
}

pub trait ScalarFrom<T>: Sized {
    fn scalar_from(value: T) -> Result<Self, TiffError>;
}
macro_rules! impl_scalar_to_floating_point_from_primitive {
    ($ttype:ty, $($ftype:ty),*) => {
        $(impl ScalarFrom<$ftype> for $ttype {
            fn scalar_from(value: $ftype) -> Result<$ttype, TiffError> {
                Ok(value as $ttype)
            }            
        })*
    }
}
macro_rules! impl_scalar_to_floating_point_from_rational {
    ($ttype:ty, $($ftype:ty),*) => {
        $(impl ScalarFrom<$ftype> for $ttype {
            fn scalar_from(value: $ftype) -> Result<$ttype, TiffError> {
                let fval = (value[0] as f64) / (value[1] as f64);
                Ok(fval as $ttype)
            }
        })*
    }
}
impl_scalar_to_floating_point_from_primitive!(f64, i8, u8, i16, u16, i32, u32, i64, u64, f32, f64);
impl_scalar_to_floating_point_from_rational!(f64, [i32;2], [u32;2]);
macro_rules! impl_scalar_from_integer {
    ($ftype:ty, $ttype:ty) => {
        impl ScalarFrom<$ftype> for $ttype {
            fn scalar_from(value: $ftype) -> Result<$ttype, TiffError> {
                <$ttype>::try_from(value).or(Err(TiffError::IncompatibleTagDataKind))
            }
        }
    };
    ($ttype:ty, $($ftype:ty),*) => {
        $(impl_scalar_from_integer!($ftype, $ttype));*;
    }
}
impl_scalar_from_integer!(i64, i8, u8, i16, u16, i32, u32, i64, u64);
impl_scalar_from_integer!(u64, i8, u8, i16, u16, i32, u32, i64, u64);
macro_rules! impl_scalar_from_floating_point {
    ($ftype:ty, $ttype:ty) => {
        impl ScalarFrom<$ftype> for $ttype {
            fn scalar_from(value: $ftype) -> Result<$ttype, TiffError> {
                if !value.is_finite() || value.fract() != 0.0 || value < <$ttype>::MIN as $ftype || value > <$ttype>::MAX as $ftype {
                    Err(TiffError::IncompatibleTagDataKind)
                } else {
                    Ok(value as $ttype)
                }
            }
        }
    };
    ($ttype:ty, $($ftype:ty),*) => {
        $(impl_scalar_from_floating_point!($ftype, $ttype));*;
    }
}
impl_scalar_from_floating_point!(i64, f32, f64);
impl_scalar_from_floating_point!(u64, f32, f64);
macro_rules! impl_scalar_from_rational {
    ($ftype:ty, $ttype:ty) => {
        impl ScalarFrom<$ftype> for $ttype {
            fn scalar_from(value:$ftype) -> Result<$ttype, TiffError> {
                let fval = (value[0] as f64) / (value[1] as f64);
                <$ttype>::scalar_from(fval)
            }
        }
    }
}
impl_scalar_from_rational!([i32;2], i64);
impl_scalar_from_rational!([i32;2], u64);
impl_scalar_from_rational!([u32;2], i64);
impl_scalar_from_rational!([u32;2], u64);

pub trait PairFromBytes<const N: usize> {
    fn from_le_bytes(data: [u8; N]) -> Self;
    fn from_be_bytes(data: [u8; N]) -> Self;
}
macro_rules! impl_pair_from_bytes {
    ($typ:ty, $N:expr, $f:ident) => {
        fn $f(data: [u8; $N]) -> Self {
            const ESIZE: usize = std::mem::size_of::<$typ>();
            let v1 = <$typ>::$f(*array_from_slice::<ESIZE>(&data[0..ESIZE]));
            let v2 = <$typ>::$f(*array_from_slice::<ESIZE>(&data[ESIZE..ESIZE * 2]));
            [v1, v2]
        }
    };
    ($typ:ty) => {
        impl<const N: usize> PairFromBytes<N> for [$typ; 2] {
            impl_pair_from_bytes!($typ, N, from_le_bytes);
            impl_pair_from_bytes!($typ, N, from_be_bytes);
        }
    }
}
impl_pair_from_bytes!(i32);
impl_pair_from_bytes!(u32);

macro_rules! define_tag_data {
    ($($name:ident, $typ:ty, $kind:expr, $size:expr),*) => {
        pub enum TagData {
            $($name(Vec<$typ>)),*
        }
        impl std::fmt::Debug for TagData {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
                match self {
                    $(Self::$name(value) => f.write_fmt(format_args!("{:?}", value))),*
                }
            }
        }
        impl Default for TagData {
            fn default() -> Self {
                Self::Byte(vec![])
            }
        }
        impl TagData {
            pub fn kind(&self) -> u16 {
                match self {
                    $(Self::$name(_) => $kind),*,
                }
            }
            pub fn new(kind: u16, count: usize) -> Result<Self, TiffError> {
                match kind {
                    $($kind => Ok(Self::$name(vec![<$typ as Default>::default(); count]))),*,
                    _ => Err(TiffError::UnknownTagKind),
                }
            }
            pub fn from_buffer(buffer: &mut BufReader<File>, btf: bool, le: bool) -> Result<Self, TiffError> {
                let kind = buffer_as!(buffer, u16, le)?;
                let count = buffer_as_offset!(buffer, btf, le) as usize;
                let byte_count = count * {
                    match kind {
                        $($kind => $size),*,
                        _ => return Err(TiffError::UnknownTagKind),
                    }
                };

                let pos = buffer.stream_position().or(Err(TiffError::UnknownBufferError))?;

                let offset_byte_count: usize = if btf { 8 } else { 4 };
                if byte_count > offset_byte_count {
                    let offset = buffer_as_offset!(buffer, btf, le);
                    buffer.seek(SeekFrom::Start(offset)).or(Err(TiffError::UnexpectedEndOfBuffer))?;
                }
                match kind {
                    $($kind => {
                        let mut data = vec![<$typ as Default>::default(); count];
                        for i in 0..count {
                            data[i] = buffer_as!(buffer, $typ, le)?;
                        }
                        buffer.seek(SeekFrom::Start(pos + offset_byte_count as u64)).or(Err(TiffError::UnknownBufferError))?;
                        return Ok(Self::$name(data));
                    }),*,
                    _ => return Err(TiffError::UnknownTagKind),
                }
            }
            pub fn as_signed_integer(&self) -> Result<i64, TiffError> {
                match &self {
                    $(Self::$name(values) => {
                        if values.len() != 1 {
                            Err(TiffError::IncompatibleTagDataKind)
                        } else {
                            i64::scalar_from(values[0])
                        }
                    }),*
                }
            }
            pub fn as_signed_integers(&self) -> Result<Vec<i64>, TiffError> {
                match &self {
                    $(Self::$name(values) => {
                        let mut results = vec![0i64; values.len()];
                        for i in 0..values.len() {
                            results[i] = i64::scalar_from(values[i])?;
                        }
                        Ok(results)
                    }),*
                }
            }
            pub fn as_unsigned_integer(&self) -> Result<u64, TiffError> {
                match &self {
                    $(Self::$name(values) => {
                        if values.len() != 1 {
                            Err(TiffError::IncompatibleTagDataKind)
                        } else {
                            u64::scalar_from(values[0])
                        }
                    }),*
                }
            }
            pub fn as_unsigned_integers(&self) -> Result<Vec<u64>, TiffError> {
                match &self {
                    $(Self::$name(values) => {
                        let mut results = vec![0u64; values.len()];
                        for i in 0..values.len() {
                            results[i] = u64::scalar_from(values[i])?;
                        }
                        Ok(results)
                    }),*
                }
            }
            pub fn as_floating_point(&self) -> Result<f64, TiffError> {
                match &self {
                    $(Self::$name(values) => {
                        if values.len() != 1 {
                            Err(TiffError::IncompatibleTagDataKind)
                        } else {
                            f64::scalar_from(values[0])
                        }
                    }),*
                }
            }
            pub fn as_floating_points(&self) -> Result<Vec<f64>, TiffError> {
                match &self {
                    $(Self::$name(values) => {
                        let mut results = vec![0.0f64; values.len()];
                        for i in 0..values.len() {
                            results[i] = f64::scalar_from(values[i])?;
                        }
                        Ok(results)
                    }),*
                }
            }
        }
    }
}
define_tag_data!(
/*--$name------$typ-----$kind---$size--*/
    Byte,      u8,      1,      1,
    Ascii,     u8,      2,      1,
    Short,     u16,     3,      2,
    Long,      u32,     4,      4,
    Rational,  [u32;2], 5,      8,
    SByte,     i8,      6,      1,
    Undefined, u8,      7,      1,
    SShort,    i16,     8,      2,
    SLong,     i32,     9,      4,
    SRational, [i32;2], 10,     8,
    Float,     f32,     11,     4,
    Double,    f64,     12,     8,
    IFD,       u32,     13,     4,
    Long8,     u64,     16,     8,
    SLong8,    i64,     17,     8,
    IFD8,      u64,     18,     8
);
macro_rules! define_tag_id {
    ($category:ident, $($id:expr, $name:ident),*) => {
        #[derive(PartialEq)]
        pub enum $category {
            $($name),*
        }
        impl std::fmt::Debug for $category {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
                match self {
                    $(Self::$name => f.write_fmt(format_args!("{}::{}({:#06x})", stringify!($category), stringify!($name), $id))),*
                }
            }
        }
        impl $category {
            pub fn from_u16(n: u16) -> Result<Self, TiffError> {
                match n {
                    $($id => Ok(Self::$name)),*,
                    _ => Err(TiffError::UnknownTagId),
                }
            }
            pub fn to_u16(&self) -> u16 {
                match self {
                    $(Self::$name => $id),*
                }
            }
        }
    }
}
define_tag_id!(Baseline,
0x00FE, NewSubfileType,
0x00FF, SubfileType,
0x0100, ImageWidth,
0x0101, ImageLength,
0x0102, BitsPerSample,
0x0103, Compression,
0x0106, PhotometricInterpretation,
0x0107, Threshholding,
0x0108, CellWidth,
0x0109, CellLength,
0x010A, FillOrder,
0x010E, ImageDescription,
0x010F, Make,
0x0110, Model,
0x0111, StripOffsets,
0x0112, Orientation,
0x0115, SamplesPerPixel,
0x0116, RowsPerStrip,
0x0117, StripByteCounts,
0x0118, MinSampleValue,
0x0119, MaxSampleValue,
0x011A, XResolution,
0x011B, YResolution,
0x011C, PlanarConfiguration,
0x0120, FreeOffsets,
0x0121, FreeByteCounts,
0x0122, GrayResponseUnit,
0x0123, GrayResponseCurve,
0x0128, ResolutionUnit,
0x0131, Software,
0x0132, DateTime,
0x013B, Artist,
0x013C, HostComputer,
0x0140, ColorMap,
0x0152, ExtraSamples,
0x8298, Copyright
);
define_tag_id!(Extension,
0x010D, DocumentName,
0x011D, PageName,
0x011E, XPosition,
0x011F, YPosition,
0x0124, T4Options,
0x0125, T6Options,
0x0129, PageNumber,
0x012D, TransferFunction,
0x013D, Predictor,
0x013E, WhitePoint,
0x013F, PrimaryChromaticities,
0x0141, HalftoneHints,
0x0142, TileWidth,
0x0143, TileLength,
0x0144, TileOffsets,
0x0145, TileByteCounts,
0x0146, BadFaxLines,
0x0147, CleanFaxData,
0x0148, ConsecutiveBadFaxLines,
0x014A, SubIFDs,
0x014C, InkSet,
0x014D, InkNames,
0x014E, NumberOfInks,
0x0150, DotRange,
0x0151, TargetPrinter,
0x0153, SampleFormat,
0x0154, SMinSampleValue,
0x0155, SMaxSampleValue,
0x0156, TransferRange,
0x0157, ClipPath,
0x0158, XClipPathUnits,
0x0159, YClipPathUnits,
0x015A, Indexed,
0x015B, JPEGTables,
0x015F, OPIProxy,
0x0190, GlobalParametersIFD,
0x0191, ProfileType,
0x0192, FaxProfile,
0x0193, CodingMethods,
0x0194, VersionYear,
0x0195, ModeNumber,
0x01B1, Decode,
0x01B2, DefaultImageColor,
0x0200, JPEGProc,
0x0201, JPEGInterchangeFormat,
0x0202, JPEGInterchangeFormatLength,
0x0203, JPEGRestartInterval,
0x0205, JPEGLosslessPredictors,
0x0206, JPEGPointTransforms,
0x0207, JPEGQTables,
0x0208, JPEGDCTables,
0x0209, JPEGACTables,
0x0211, YCbCrCoefficients,
0x0212, YCbCrSubSampling,
0x0213, YCbCrPositioning,
0x0214, ReferenceBlackWhite,
0x022F, StripRowCounts,
0x02BC, XMP,
0x800D, ImageID,
0x87AC, ImageLayer
);
#[derive(PartialEq)]
pub enum TagID {
    PrivateTag(u16),
    BaselineTag(Baseline),
    ExtensionTag(Extension),
}
impl Default for TagID {
    fn default() -> Self {
        Self::PrivateTag(0)
    }
}
impl std::fmt::Debug for TagID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::PrivateTag(id) => f.write_fmt(format_args!("PrivateTag({:#06x})", id)),
            Self::BaselineTag(id) => f.write_fmt(format_args!("{:#?}", id)),
            Self::ExtensionTag(id) => f.write_fmt(format_args!("{:#?}", id)),
        }
    }
}
impl TagID {
    pub fn from_u16(n: u16) -> Self {
        if let Ok(value) = Baseline::from_u16(n) {
            return Self::BaselineTag(value);
        }
        if let Ok(value) = Extension::from_u16(n) {
            return Self::ExtensionTag(value);
        }
        Self::PrivateTag(n)
    }
}
#[derive(Default)]
pub struct Tag {
    id: TagID,
    data: TagData,
}
impl std::fmt::Debug for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("Tag")
         .field("id", &self.id)
         .field("data", &self.data)
         .finish()
    }
}
impl Tag {
    pub fn from_buffer(buffer: &mut BufReader<File>, btf: bool, le: bool) -> Result<Self, TiffError> {
        let id = TagID::from_u16(buffer_as!(buffer, u16, le)?);
        let data = TagData::from_buffer(buffer, btf, le)?;
        Ok(Self{id, data})
    }
}

#[derive(Debug, Default)]
pub struct IFD {
    pub pos: u64,
    pub tag_count: u64,
    pub tags: Vec<Tag>,
    pub next_ifd: u64,
}
impl IFD {
    pub fn from_buffer(buffer: &mut BufReader<File>, btf: bool, le: bool, skip: bool) -> Result<Self, TiffError> {
        let pos = buffer.stream_position().or(Err(TiffError::UnknownBufferError))?;
        let tag_count = if btf {
            buffer_as!(buffer, u64, le)?
        } else {
            buffer_as!(buffer, u16, le)? as u64
        };
        let mut tags: Vec<Tag> = vec![];
        if skip {
            let tag_byte_count = if btf {
                tag_count * 20
            } else {
                tag_count * 12
            };
            buffer.seek(SeekFrom::Current(tag_byte_count as i64)).or(Err(TiffError::UnexpectedEndOfBuffer))?;
        } else {
            for _ in 0..tag_count {
                let tag = Tag::from_buffer(buffer, btf, le)?;
                tags.push(tag);
            }
        }
        let next_ifd = buffer_as_offset!(buffer, btf, le);

        Ok(Self{pos, tag_count, tags, next_ifd})
    }
    pub fn get_tag(&self, id: TagID) -> Result<&Tag, TiffError> {
        for tag in &self.tags {
            if id == tag.id {
                return Ok(tag);
            }
        }
        return Err(TiffError::CannotFindTag);
    }
    pub fn width(&self) -> Result<u64, TiffError> {
        let tag = self.get_tag(TagID::BaselineTag(Baseline::ImageWidth))?;
        tag.data.as_unsigned_integer()
    }
    pub fn height(&self) -> Result<u64, TiffError> {
        let tag = self.get_tag(TagID::BaselineTag(Baseline::ImageLength))?;
        tag.data.as_unsigned_integer()
    }
    pub fn samples(&self) -> Result<u64, TiffError> {
        // default: 1
        if let Ok(tag) = self.get_tag(TagID::BaselineTag(Baseline::SamplesPerPixel)) {
            return tag.data.as_unsigned_integer();
        }
        Ok(1_u64)
    }
    pub fn bpp(&self) -> Result<Vec<u64>, TiffError> {
        if let Ok(tag) = self.get_tag(TagID::BaselineTag(Baseline::BitsPerSample)) {
            return tag.data.as_unsigned_integers();
        }
        return Ok(vec![1_u64; self.samples()? as usize]);
    }
    pub fn compression(&self) -> Result<CompressionScheme, TiffError> {
        // default: 1 (no compression)
        if let Ok(tag) = self.get_tag(TagID::BaselineTag(Baseline::Compression)) {
            match CompressionScheme::from_number(tag.data.as_unsigned_integer()?) {
                Some(value) => return Ok(value),
                None => return Err(TiffError::NotSupportedCompressionScheme),
            }
        }
        Ok(CompressionScheme::NoCompression)
    }
}
