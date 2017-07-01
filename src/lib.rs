#[macro_use]
extern crate nom;
use nom::{be_u32, be_u8};

extern crate crc;
use crc::crc32;

use std::mem::transmute;

#[derive(Debug)]
struct IHDR {
    width: u32,
    height: u32,
    bit_depth: u8,
    color_type: u8,
    compression: u8,
    filter: u8,
    interlace: u8,
}

#[derive(Debug)]
struct TEXT {
    keyword: String,
    text: String,
}

#[derive(Debug)]
struct ZTXT {
    keyword: String,
    compression: u8,
    text: Vec<u8>,
}

#[derive(Debug)]
enum ChunkData {
    IHDR(IHDR),
    TEXT(TEXT),
    ZTXT(ZTXT),
    UNKNOWN(Vec<u8>),
}

#[derive(Debug)]
struct Chunk {
    chunk_type: [u8; 4],
    chunk_data: ChunkData,
    crc: u32,
    computed: u32,
}

#[derive(Debug)]
struct PNG {
    chunks: Vec<Chunk>,
}

named!(header < &[u8] >,
    tag!(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])
);

named!(ihdr < ChunkData >,
    do_parse!(
        width: be_u32       >>
        height: be_u32      >>
        bit_depth: be_u8    >>
        color_type: be_u8   >>
        compression: be_u8  >>
        filter: be_u8       >>
        interlace: be_u8    >>
        (
            ChunkData::IHDR(
                IHDR {
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
    )
);

fn text(input: &[u8], length: usize) -> ::nom::IResult<&[u8], ChunkData> {
    do_parse!(input,
        keyword: take_until!("\0") >>
        tag!("\0") >>
        data: take!(length - keyword.len() - 1) >>
        (
            ChunkData::TEXT(
                TEXT {
                    keyword: String::from_utf8_lossy(keyword).into_owned(),
                    text: String::from_utf8_lossy(data).into_owned(),
                }
            )
        )
    )
}

fn ztxt(input: &[u8], length: usize) -> ::nom::IResult<&[u8], ChunkData> {
    do_parse!(input,
        keyword: take_until!("\0") >>
        tag!("\0") >>
        compression: be_u8 >>
        data: take!(length as usize - keyword.len() - 2) >>
        (
            ChunkData::ZTXT(
                ZTXT {
                    keyword: String::from_utf8_lossy(keyword).into_owned(),
                    compression,
                    text: data.into(),
                }
            )
        )
    )
}

named!(chunk < Chunk >,
    do_parse!(
        length: be_u32                  >>
        chunk_type: peek!(take!(4))     >>
        computed: peek!(take!(length + 4)) >>
        chunk_data: switch!(take!(4),
            b"IHDR" => call!(ihdr) |
            b"tEXt" => apply!(text, length as usize) |
            b"zTXt" => apply!(ztxt, length as usize) |
            _ => do_parse!(
                data: take!(length) >>
                (
                    ChunkData::UNKNOWN(data.into())
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
                crc,
                computed: crc32::checksum_ieee(computed),
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
        let (_, png) = png(data.as_slice()).unwrap();

        for chunk in png.chunks {
            println!("{:?}, crc: {:?}, computed: {:?}", chunk.chunk_type, chunk.crc, chunk.computed);
        }
    }
}
