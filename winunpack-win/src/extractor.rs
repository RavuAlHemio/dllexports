use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use windows::Win32::Foundation::S_FALSE;
use windows_core::{implement, OutRef};
use z7_com::{
    IArchiveExtractCallback, IArchiveExtractCallback_Impl, IOutStream, IProgress_Impl,
    ISequentialOutStream,
};

use crate::rust_7z_stream::{MemorySequentialOutStream, Rust7zOutStream};


#[implement(IArchiveExtractCallback)]
pub struct SingleFileExtractor {
    index: u32,
    destination: Mutex<Option<File>>,
}
impl SingleFileExtractor {
    pub fn new(index: u32, destination: File) -> Self {
        Self {
            index,
            destination: Mutex::new(Some(destination)),
        }
    }

    pub fn index(&self) -> u32 { self.index }
    pub fn into_inner(self) -> Mutex<Option<File>> { self.destination }
}
impl IProgress_Impl for SingleFileExtractor_Impl {
    fn SetTotal(&self, _total: u64) -> windows_core::Result<()> {
        Ok(())
    }

    fn SetCompleted(&self, _complete_value: *const u64) -> windows_core::Result<()> {
        Ok(())
    }
}
impl IArchiveExtractCallback_Impl for SingleFileExtractor_Impl {
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


#[implement(IArchiveExtractCallback)]
pub struct SingleFileToMemoryExtractor {
    index: u32,
    destination: Arc<Mutex<Vec<u8>>>,
}
impl SingleFileToMemoryExtractor {
    pub fn new(index: u32, destination: Arc<Mutex<Vec<u8>>>) -> Self {
        Self {
            index,
            destination,
        }
    }

    pub fn index(&self) -> u32 { self.index }
    pub fn destination(&self) -> Arc<Mutex<Vec<u8>>> { Arc::clone(&self.destination) }
}
impl IProgress_Impl for SingleFileToMemoryExtractor_Impl {
    fn SetTotal(&self, _total: u64) -> windows_core::Result<()> {
        Ok(())
    }

    fn SetCompleted(&self, _complete_value: *const u64) -> windows_core::Result<()> {
        Ok(())
    }
}
impl IArchiveExtractCallback_Impl for SingleFileToMemoryExtractor_Impl {
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

        let mem_out_stream = MemorySequentialOutStream::new(Arc::clone(&self.destination));
        let seq_stream = ISequentialOutStream::from(mem_out_stream);
        out_stream.write(Some(seq_stream))
            .expect("failed to set output stream");
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


#[implement(IArchiveExtractCallback)]
pub struct MultiFileExtractor {
    parent_dir_path: PathBuf,
    index_to_extract_path: BTreeMap<u32, String>,
}
impl MultiFileExtractor {
    pub fn new(
        parent_dir_path: PathBuf,
        index_to_extract_path: BTreeMap<u32, String>,
    ) -> Self {
        Self {
            parent_dir_path,
            index_to_extract_path,
        }
    }

    pub fn parent_dir_path(&self) -> &Path { &self.parent_dir_path }
    pub fn index_to_extract_path(&self) -> &BTreeMap<u32, String> { &self.index_to_extract_path }
}
impl IProgress_Impl for MultiFileExtractor_Impl {
    fn SetTotal(&self, _total: u64) -> windows_core::Result<()> {
        Ok(())
    }

    fn SetCompleted(&self, _complete_value: *const u64) -> windows_core::Result<()> {
        Ok(())
    }
}
impl IArchiveExtractCallback_Impl for MultiFileExtractor_Impl {
    fn GetStream(
        &self,
        index: u32,
        out_stream: OutRef<ISequentialOutStream>,
        _ask_extract_mode: i32,
    ) -> windows_core::Result<()> {
        let Some(extract_path) = self.index_to_extract_path.get(&index) else {
            // not interested
            return Err(windows_core::Error::from(S_FALSE));
        };
        let mut dest_path = self.parent_dir_path.clone();
        for piece in extract_path.split("\\") {
            dest_path.push(piece);
        }

        println!("{}", dest_path.display());

        let dest_dir = dest_path.parent().unwrap();
        let dest_dir_exists = std::fs::exists(dest_dir)
            .expect("failed to check existence of output dir");
        if !dest_dir_exists {
            std::fs::create_dir_all(dest_dir)
                .expect("failed to create output dir");
        }

        let inner_file = File::create(&dest_path)
            .expect("failed to create output file");

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
