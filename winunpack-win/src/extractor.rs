use std::fs::File;
use std::sync::Mutex;

use windows::Win32::Foundation::S_FALSE;
use windows_core::{implement, OutRef};
use z7_com::{
    IArchiveExtractCallback, IArchiveExtractCallback_Impl, IOutStream, IProgress_Impl,
    ISequentialOutStream,
};

use crate::rust_7z_stream::Rust7zOutStream;


#[implement(IArchiveExtractCallback)]
pub struct Extractor {
    index: u32,
    destination: Mutex<Option<File>>,
}
impl Extractor {
    pub fn new(index: u32, destination: File) -> Self {
        Self {
            index,
            destination: Mutex::new(Some(destination)),
        }
    }

    pub fn index(&self) -> u32 { self.index }
    pub fn into_inner(self) -> Mutex<Option<File>> { self.destination }
}
impl IProgress_Impl for Extractor_Impl {
    fn SetTotal(&self, _total: u64) -> windows_core::Result<()> {
        Ok(())
    }

    fn SetCompleted(&self, _complete_value: *const u64) -> windows_core::Result<()> {
        Ok(())
    }
}
impl IArchiveExtractCallback_Impl for Extractor_Impl {
    fn GetStream(
        &self,
        index: u32,
        out_stream: OutRef<ISequentialOutStream>,
        _ask_extract_mode: i32,
    ) -> windows_core::Result<()> {
        if index != self.index {
            // not interested
            return Err(windows_core::Error::from(S_FALSE));
        }

        let inner_file_opt = {
            let mut guard = self.destination
                .lock().expect("failed to lock mutex");
            guard.take()
        };
        let Some(inner_file) = inner_file_opt else {
            return Err(windows_core::Error::from(S_FALSE));
        };

        let rust_out_stream = Rust7zOutStream::new(Mutex::new(Box::new(
            inner_file,
        )));
        let isos = ISequentialOutStream::from(IOutStream::from(rust_out_stream));
        out_stream.write(Some(isos))?;
        Ok(())
    }

    fn PrepareOperation(&self, _ask_extract_mode: i32) -> windows_core::Result<()> {
        Ok(())
    }

    fn SetOperationResult(
        &self,
        _opres: &z7_com::EExtractOperationResult,
    ) -> windows_core::Result<()> {
        Ok(())
    }
}
