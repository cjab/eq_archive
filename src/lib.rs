//! # An Everquest archive file extractor
//! This has only been tested on .s3d files and implements only the bare minimum of functionality.
//! CRC checks for example are completely ignored.
//!
//! # Examples
//! ```rust
//! let archive = eq_archive::read("gfaydark.s3d").unwrap();
//!
//! // List all files in the archive
//! let filenames = archive.filenames();
//!
//! // Iterate over files in the archive
//! for (name, data) in archive.files() {
//!
//! }
//!
//! ```
//!

mod parser;

pub use parser::{Archive, Entry};

use std::fs::File;
use std::io::{self, Read};

use flate2::read::ZlibDecoder;
use nom::error::ErrorKind;

impl Archive {
    pub fn filenames(&self) -> Vec<String> {
        let directory = self.entries.last().expect("Directory block does not exist");
        let uncompressed_blocks = directory.decompress();
        let (_, filenames) =
            parser::directory(&uncompressed_blocks[..]).expect("Failed to parse directory block");
        filenames
    }

    pub fn get(&self, filename: &str) -> Option<Vec<u8>> {
        self.filenames()
            .iter()
            .position(|f| f.eq_ignore_ascii_case(filename))
            .and_then(|position| self.entries.get(position).map(|entry| entry.decompress()))
    }

    pub fn files(self) -> impl Iterator<Item = (String, Vec<u8>)> {
        self.filenames()
            .into_iter()
            .zip(self.entries.into_iter().map(|entry| entry.decompress()))
    }
}

impl Entry {
    fn decompress(&self) -> Vec<u8> {
        self.blocks
            .as_ref()
            .expect("Failed to decompress block")
            .iter()
            .flat_map(|block| {
                let mut buf = Vec::new();
                ZlibDecoder::new(&block.data[..])
                    .read_to_end(&mut buf)
                    .expect("Failed to decompress block");
                buf
            })
            .collect()
    }
}
#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    Parser,
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::IO(e)
    }
}

impl From<nom::Err<(&[u8], ErrorKind)>> for Error {
    fn from(_: nom::Err<(&[u8], ErrorKind)>) -> Self {
        Self::Parser
    }
}

pub fn read(filename: &str) -> Result<Archive, Error> {
    let buffer = fill_buffer(filename)?;
    Ok(parser::parse(&buffer[..])?)
}

pub fn load(data: &[u8]) -> Result<Archive, Error> {
    Ok(parser::parse(&data[..])?)
}

fn fill_buffer(filename: &str) -> io::Result<Vec<u8>> {
    let mut file = File::open(filename)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
