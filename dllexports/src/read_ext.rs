use std::io::{self, Read};


pub trait ReadExt {
    fn read_exact_or_eof(&mut self, buf: &mut [u8]) -> Result<usize, io::Error>;
}
impl<R: Read> ReadExt for R {
    fn read_exact_or_eof(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        let mut total_bytes_read = 0;
        while total_bytes_read < buf.len() {
            let bytes_read_this_time = self.read(&mut buf[total_bytes_read..])?;
            if bytes_read_this_time == 0 {
                // EOF, break out
                break;
            }
            total_bytes_read += bytes_read_this_time;
        }
        Ok(total_bytes_read)
    }
}
