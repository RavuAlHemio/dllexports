use std::ffi::CString;
use std::fs::File;
use std::path::PathBuf;

use libc::{close, mkstemp};


#[derive(Debug, Eq, PartialEq)]
pub struct TempFile {
    path: PathBuf,
}
impl TempFile {
    pub fn create() -> Self {
        // find the location of the temp directory
        let mut tmp_path = if let Some(s) = std::env::var_os("TMPDIR") {
            PathBuf::from(s)
        } else {
            PathBuf::from("/tmp")
        };
        tmp_path.push("winunpackXXXXXX");
        let mut tmp_path_c = CString::from(tmp_path);

        // create the temp file
        let temp_fd = unsafe {
            mkstemp(tmp_path_c.as_mut_ptr())
        };
        if temp_fd == -1 {
            panic!("failed to create temporary file: {}", std::io::Error::last_os_error());
        }

        let actual_tmp_path = PathBuf::from(tmp_path_c);

        // close it
        let status = unsafe {
            close(temp_fd);
        };
        if status != 0 {
            let last_error = std::io::Error::last_os_error();
            let _ = std::fs::remove_file(&actual_tmp_path);
            panic!("failed to close temporary file: {}", last_error);
        }

        Self {
            path: actual_tmp_path,
        }
    }

    pub fn path(&self) -> PathBuf { self.path.clone() }

    pub fn open_to_write(&self) -> File {
        File::create(&self.path)
            .expect("failed to open temporary file to write")
    }

    pub fn open_to_read(&self) -> File {
        File::open(&self.path)
            .expect("failed to open temporary file to read")
    }
}
impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
