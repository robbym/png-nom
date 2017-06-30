#[macro_use]
extern crate nom;
use nom::be_u32;
use nom::be_u8;

extern crate inflate;
use inflate::inflate_bytes_zlib;

use std::mem::transmute;

#[derive(Debug)]
struct ImageHeader {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    compression: u8,
    filter: u8,
    interlace: u8,
}

#[derive(Debug)]
struct TextData {
    keyword: String,
    text: String,
}

#[derive(Debug)]
enum ChunkData {
    Header(ImageHeader),
    TextData(TextData),
    Unknown(Vec<u8>),
}

#[derive(Debug)]
struct Chunk {
    chunk_type: [u8; 4],
    chunk_data: ChunkData,
    crc: u32,
}

#[derive(Debug)]
struct PNG {
    chunks: Vec<Chunk>,
}

named!(header < &[u8] >,
    tag!(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])
);

named!(chunk < Chunk >,
    do_parse!(
        length: be_u32                  >>
        chunk_type: peek!(take!(4))     >>
        chunk_data: switch!(take!(4),
            b"IHDR" => do_parse!(
                width: be_u32       >>
                height: be_u32      >>
                bit_depth: be_u8    >>
                color_type: be_u8   >>
                compression: be_u8  >>
                filter: be_u8       >>
                interlace: be_u8    >>
                (
                    ChunkData::Header(
                        ImageHeader {
                            width,
                            height,
                            bit_depth,
                            color_type,
                            compression,
                            filter,
                            interlace,
                        }
                    )
                )
            ) |
            b"tEXt" => do_parse!(
                keyword: take_until!("\0") >>
                text: take!(length as usize - keyword.len()) >>
                (
                    ChunkData::TextData(
                        TextData {
                            keyword: String::from_utf8_lossy(keyword).into_owned(),
                            text: String::from_utf8_lossy(text).into_owned(),
                        }
                    )
                )
            ) |
            b"zTXt" => do_parse!(
                keyword: take_until!("\0") >>
                tag!("\0") >>
                tag!("\0") >>
                data: take!(length as usize - keyword.len() - 2) >>
                (
                    ChunkData::TextData(
                        TextData {
                            keyword: String::from_utf8_lossy(keyword).into_owned(),
                            text: String::from_utf8_lossy(&inflate_bytes_zlib(data).unwrap()).into_owned(),
                        }
                    )
                )
            ) |
            _ => do_parse!(
                data: take!(length) >>
                (
                    ChunkData::Unknown(data.into())
                )
            )
        )                               >>
        crc: be_u32                     >>
        (
            Chunk {
                chunk_type: [
                    chunk_type[0],
                    chunk_type[1],
                    chunk_type[2],
                    chunk_type[3],
                ],
                chunk_data: chunk_data,
                crc
            }
        )
    )
);

named!(png < PNG >,
    do_parse!(
        call!(header)           >>
        chunks: many1!(chunk)   >>
        (PNG { chunks })
    )
);

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use super::*;
        use std::fs::File;
        use std::io::Read;

        let mut data = vec![];
        let mut dmi = File::open("./floors.dmi").unwrap();
        dmi.read_to_end(&mut data);
        let res = png(data.as_slice());
        println!("{:?}", res);
    }
}
