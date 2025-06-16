use std::io::Cursor;

use crate::data_mgmt::SingleFileContainer;


#[derive(Debug)]
pub(crate) struct KwajOrSz {
    compressed_data: Vec<u8>,
}
impl KwajOrSz {
    pub fn new<B: Into<Vec<u8>>>(compressed_data: B) -> Self {
        Self {
            compressed_data: compressed_data.into(),
        }
    }
}
impl SingleFileContainer for KwajOrSz {
    fn read_file(&self) -> Result<Vec<u8>, crate::data_mgmt::Error> {
        let mut reader = Cursor::new(&self.compressed_data);
        let mut decompressed_data = Vec::new();
        expandms::decompress(&mut reader, &mut decompressed_data)?;
        Ok(decompressed_data)
    }
}
