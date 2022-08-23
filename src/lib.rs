#![allow(unused_imports)]

mod error;
use error::TiffError;

#[macro_use]
mod utils;

mod enums;
use enums::{CompressionScheme};

mod header;
use header::Header;

mod ifd;
use ifd::IFD;

mod tiff;
use tiff::Tiff;



#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::fs::File;
    use std::io::BufReader;
    #[test]
    fn it_works() {
        let path = env!("CARGO_MANIFEST_DIR");
        let path = Path::new(path);
        let path = path.join("samples/BigTIFF.tif");
        let tiff = super::Tiff::from_path(&path, true).unwrap();
        println!("{:#?}", tiff);
        let n = tiff.len();
        for i in 0..n {
            let ifd = tiff.read_frame(i).unwrap();
            let width = ifd.width().unwrap();
            let height = ifd.height().unwrap();
            let samples = ifd.samples().unwrap();
            let bpp = ifd.bpp().unwrap();
            let compression = ifd.compression().unwrap();
            println!("Image Width: {}", width);
            println!("Image Height: {}", height);
            println!("Samples per Pixel: {}", samples);
            println!("Bits per Sample: {:?}", bpp);
            println!("Compression: {:?}", compression);
        }
    }
}
