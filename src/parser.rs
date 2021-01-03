use nom::bytes::complete::take;
use nom::error::ErrorKind;
use nom::multi::{count, length_data};
use nom::number::complete::le_u32;
use nom::sequence::tuple;
use nom::IResult;

const HEADER_SIZE: u32 = 12;
const BLOCK_HEADER_SIZE: u32 = 8;

pub fn parse(data: &[u8]) -> Result<Archive, nom::Err<(&[u8], ErrorKind)>> {
    let (_, archive) = archive(&data[..])?;
    Ok(archive)
}

#[derive(Debug)]
pub struct Header {
    pointer: u32,
    magic_number: u32,
    version: u32,
}

fn header(input: &[u8]) -> IResult<&[u8], Header> {
    let (remaining, (pointer, magic_number, version)) = tuple((le_u32, le_u32, le_u32))(input)?;
    Ok((
        remaining,
        Header {
            pointer,
            magic_number,
            version,
        },
    ))
}

#[derive(Debug)]
pub struct Entry {
    filename_crc: u32,
    pointer: u32,
    pub uncompressed_size: u32,
    pub blocks: Option<Vec<Block>>,
}

fn entry(input: &[u8]) -> IResult<&[u8], Entry> {
    let (remaining, (filename_crc, pointer, uncompressed_size)) =
        tuple((le_u32, le_u32, le_u32))(input)?;
    Ok((
        remaining,
        Entry {
            filename_crc,
            pointer,
            uncompressed_size,
            blocks: None,
        },
    ))
}

#[derive(Debug, PartialEq)]
pub struct Block {
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub data: Vec<u8>,
}

pub fn block(input: &[u8]) -> IResult<&[u8], Block> {
    let (i, (compressed_size, uncompressed_size)) = tuple((le_u32, le_u32))(input)?;
    let (remaining, data) = take(compressed_size)(i)?;
    Ok((
        remaining,
        Block {
            compressed_size,
            uncompressed_size,
            data: Vec::from(data),
        },
    ))
}

#[derive(Debug)]
pub struct Footer {
    footer_string: Vec<u8>,
    timestamp: u32,
}

fn footer(input: &[u8]) -> IResult<&[u8], Footer> {
    let (remaining, (footer_string, timestamp)) = tuple((take(5usize), le_u32))(input)?;
    Ok((
        remaining,
        Footer {
            footer_string: Vec::from(footer_string),
            timestamp,
        },
    ))
}

#[derive(Debug)]
pub struct Archive {
    header: Header,
    entry_count: u32,
    pub entries: Vec<Entry>,
    footer: Footer,
}

fn archive(input: &[u8]) -> IResult<&[u8], Archive> {
    let (i, header) = header(input)?;
    let (i, (data, entry_count)) = tuple((take(header.pointer - HEADER_SIZE), le_u32))(i)?;
    let (remaining, (mut entries, footer)) =
        tuple((count(entry, entry_count as usize), footer))(i)?;

    entries.sort_by_key(|a| a.pointer);

    let entries: Vec<Entry> = entries
        .into_iter()
        .map(|mut e| {
            let mut offset = (e.pointer - HEADER_SIZE) as usize;
            let mut bytes_remaining = e.uncompressed_size;
            let mut blocks = Vec::new();

            while bytes_remaining > 0 {
                let b = block(&data[offset..]).expect("Error parsing block").1;
                offset += (BLOCK_HEADER_SIZE + b.compressed_size) as usize;
                bytes_remaining -= b.uncompressed_size;
                blocks.push(b);
            }
            e.blocks = Some(blocks);
            e
        })
        .collect();

    Ok((
        remaining,
        Archive {
            header,
            entry_count,
            entries,
            footer,
        },
    ))
}

fn directory_string(input: &[u8]) -> IResult<&[u8], String> {
    let (remaining, data) = length_data(le_u32)(input)?;
    Ok((
        remaining,
        String::from_utf8(Vec::from(data))
            .unwrap()
            // Strings stored in directory are null terminated
            .trim_end_matches('\0')
            .to_string(),
    ))
}

pub fn directory(input: &[u8]) -> IResult<&[u8], Vec<String>> {
    let (i, file_count) = le_u32(input)?;
    let (remaining, filenames) = count(directory_string, file_count as usize)(i)?;
    Ok((remaining, filenames.to_vec()))
}
