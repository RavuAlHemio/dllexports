use std::fs::File;
use std::os::windows::io::FromRawHandle;
use std::sync::LazyLock;

use windows::core::{PCWSTR, s, w};
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_DELETE_ON_CLOSE, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE, GetTempFileNameW, OPEN_EXISTING,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::SystemServices::UNICODE_STRING_MAX_CHARS;


/// The function GetTempPath2W on systems which support it and GetTempPathW otherwise.
static GET_TEMP_PATH_W: LazyLock<unsafe extern "system" fn(buffer_length: u32, buffer: *mut u16) -> u32> = LazyLock::new(|| {
    // we probably hard-depend on kernel32 anyway
    let kernel32_handle = unsafe {
        GetModuleHandleW(w!("kernel32.dll"))
    }
        .expect("failed to obtain handle for kernel32.dll");
    unsafe {
        GetProcAddress(kernel32_handle, s!("GetTempPath2W"))
            .or_else(|| GetProcAddress(kernel32_handle, s!("GetTempPathW")))
            .map(|f| std::mem::transmute(f))
            .expect("GetTempPathW has been available since NT3.5 and Win95, what gives?")
    }
});


#[derive(Debug, Eq, PartialEq)]
pub struct TempFile {
    path_nul_terminated: Vec<u16>,
}
impl TempFile {
    pub fn create() -> Self {
        let buf_size_u32 = UNICODE_STRING_MAX_CHARS;
        let buf_size_usize: usize = buf_size_u32.try_into().unwrap();
        let mut buf = vec![0u16; buf_size_usize];

        // find the location of the temp directory
        let temp_count = unsafe {
            GET_TEMP_PATH_W(buf_size_u32, buf.as_mut_ptr())
        };
        if temp_count == 0 {
            panic!("failed to obtain temp directory: {}", windows_core::Error::from_thread());
        }
        let temp_count_usize: usize = temp_count.try_into().unwrap();
        buf.drain(temp_count_usize..);
        buf.push(0x0000);

        let mut temp_file_name = [0u16; 260];

        // create a temp file there
        let temp_number = unsafe {
            GetTempFileNameW(
                PCWSTR(buf.as_ptr()),
                w!("wup"),
                0,
                &mut temp_file_name,
            )
        };
        if temp_number == 0 {
            panic!("failed to create temporary file: {}", windows_core::Error::from_thread());
        }
        let nul_pos = temp_file_name.iter()
            .position(|w| *w == 0x0000)
            .unwrap_or(temp_file_name.len());
        let mut actual_temp_file_name = temp_file_name[0..nul_pos].to_vec();
        actual_temp_file_name.push(0x0000);

        Self {
            path_nul_terminated: actual_temp_file_name,
        }
    }

    pub fn path_nul_terminated(&self) -> &[u16] { &self.path_nul_terminated }

    fn open_again(&self, access: u32) -> File {
        // open a fresh handle to the temp file
        let temp_file_handle = unsafe {
            CreateFileW(
                PCWSTR(self.path_nul_terminated.as_ptr()),
                access,
                FILE_SHARE_DELETE | FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        }
            .expect("failed to open temp file");
        unsafe {
            File::from_raw_handle(temp_file_handle.0)
        }
    }

    pub fn open_to_write(&self) -> File {
        self.open_again((GENERIC_READ | GENERIC_WRITE).0)
    }

    pub fn open_to_read(&self) -> File {
        self.open_again(GENERIC_READ.0)
    }
}
impl Drop for TempFile {
    fn drop(&mut self) {
        // mark the file for deletion -- hopefully this covers most cases
        let temp_file_handle = unsafe {
            CreateFileW(
                PCWSTR(self.path_nul_terminated.as_ptr()),
                0,
                FILE_SHARE_DELETE | FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL | FILE_FLAG_DELETE_ON_CLOSE,
                None,
            )
        }
            .expect("failed to open temp file");
        let _ = unsafe {
            CloseHandle(temp_file_handle)
        };
    }
}
