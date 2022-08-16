#![allow(dead_code)]

use std::path::Path;
use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use crate::{TiffError, Header, IFD};

#[derive(Debug, Default)]
pub struct Tiff {
    pub path: String,
    pub header: Header,
    pub ifd: Vec<IFD>,
}
impl Tiff {
    pub fn from_path(path: &Path, skip: bool) -> Result<Self, TiffError> {
        let file = File::open(path).or(Err(TiffError::CannotOpenFile))?;
        let mut buffer = BufReader::new(file);
        let header = Header::from_buffer(&mut buffer)?;
        let btf = header.is_btf();
        let le = header.is_le();
        let mut next_ifd = header.first_ifd;
        let mut ifd: Vec<IFD> = vec![];
        while next_ifd != 0 {
            buffer.seek(SeekFrom::Start(next_ifd)).or(Err(TiffError::UnexpectedEndOfBuffer))?;
            let directory = IFD::from_buffer(&mut buffer, btf, le, skip)?;
            next_ifd = directory.next_ifd;
            ifd.push(directory);
        }
        Ok(Self{path: String::from(path.to_str().unwrap_or("")), header, ifd})
    }
    pub fn len(&self) -> usize {
        self.ifd.len()
    }
    pub fn read_frame(&self, index: usize) -> Result<IFD, TiffError> {
        if index >= self.len() {
            return Err(TiffError::InvalidIndex);
        }
        let file = File::open(&self.path).or(Err(TiffError::CannotOpenFile))?;
        let mut buffer = BufReader::new(file);
        let btf = self.header.is_btf();
        let le = self.header.is_le();
        let pos = self.ifd[index].pos;
        buffer.seek(SeekFrom::Start(pos)).or(Err(TiffError::UnexpectedEndOfBuffer))?;
        let directory = IFD::from_buffer(&mut buffer, btf, le, false)?;
        Ok(directory)
    }
}
