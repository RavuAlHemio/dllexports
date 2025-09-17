pub mod bitmap;
pub mod bitmap_font;
pub mod code_view;
pub mod ico1;
pub mod icon_group;
mod int_from_byte_slice;
#[macro_use] pub(crate) mod macros;
pub mod mz;
pub mod ne;
pub mod nt4dbg;
pub mod part_int;
pub mod pe;


use std::io::{self, Read};

use tracing::debug;


pub(crate) fn read_nul_terminated_ascii_string<R: Read>(reader: &mut R) -> Result<String, io::Error> {
    let mut buf = [0u8];
    let mut ret = Vec::new();
    loop {
        reader.read_exact(&mut buf)?;
        if buf[0] == 0x00 {
            break;
        }
        ret.push(buf[0]);
    }
    String::from_utf8(ret)
        .inspect_err(|_| debug!("NUL-terminated string is invalid UTF-8"))
        .map_err(|_| io::ErrorKind::InvalidData.into())
}

pub(crate) fn collect_nul_terminated_ascii_string(bytes: &[u8]) -> Option<String> {
    let nul_pos = bytes
        .iter()
        .position(|b| *b == 0x00)
        .unwrap_or(bytes.len());
    let subslice = &bytes[..nul_pos];
    std::str::from_utf8(subslice)
        .ok()
        .map(|s| s.to_owned())
}

/// Reads a UTF-16LE string that is prefixed by a u16le length.
pub(crate) fn read_pascal_utf16le_string<R: Read>(reader: &mut R) -> Result<String, io::Error> {
    let mut length_buf = [0u8; 2];
    reader.read_exact(&mut length_buf)?;
    let length_chars_u16 = u16::from_le_bytes(length_buf);

    let mut string_bytes = vec![0u8; usize::from(length_chars_u16) * 2];
    reader.read_exact(&mut string_bytes)?;

    let mut words = Vec::with_capacity(string_bytes.len() / 2);
    for word_bytes in string_bytes.chunks(2) {
        let word = u16::from_le_bytes(word_bytes.try_into().unwrap());
        words.push(word);
    }
    String::from_utf16(&words)
        .inspect_err(|_| debug!("Pascal little-endian wide string is invalid UTF-16"))
        .map_err(|_| io::ErrorKind::InvalidData.into())
}

/// Reads a byte string that is prefixed by a u8 length.
pub(crate) fn read_pascal_byte_string<R: Read>(reader: &mut R) -> Result<Vec<u8>, io::Error> {
    let mut length_buf = [0u8; 1];
    reader.read_exact(&mut length_buf)?;

    let mut string_bytes = vec![0u8; usize::from(length_buf[0])];
    reader.read_exact(&mut string_bytes)?;
    Ok(string_bytes)
}
