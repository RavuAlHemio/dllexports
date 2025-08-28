pub mod bitmap;
pub mod ico1;
pub mod icon_group;
pub mod mz;
pub mod ne;
pub mod pe;


use std::io::{self, Read};


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
        .map_err(|_| io::ErrorKind::InvalidData.into())
}
