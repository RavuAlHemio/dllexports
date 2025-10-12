use std::io::{Read, Seek, SeekFrom};
use std::sync::Mutex;

use windows::Win32::Foundation::ERROR_INVALID_PARAMETER;
use windows::Win32::System::Com::{STREAM_SEEK_CUR, STREAM_SEEK_END, STREAM_SEEK_SET};
use windows_implement::implement;
use z7_com::{IInStream, IInStream_Impl, ISequentialInStream, ISequentialInStream_Impl};


macro_rules! impl_read {
    () => {
        fn Read(
            &self,
            data: *mut core::ffi::c_void,
            size: u32,
            processed_size: *mut u32,
        ) -> windows_core::Result<()> {
            let data_u8 = data as *mut u8;
            let size_usize: usize = size.try_into().unwrap();
            let data_slice = unsafe {
                std::slice::from_raw_parts_mut(data_u8, size_usize)
            };
            let processed_size_ref = unsafe { processed_size.as_mut() }
                .expect("failed to convert processed size pointer to reference");

            let res = {
                let mut guard = self.inner
                    .lock().expect("locking mutex failed");
                guard.read(data_slice)
            };
            match res {
                Ok(processed_usize) => {
                    *processed_size_ref = processed_usize.try_into().unwrap();
                    Ok(())
                },
                Err(e) => Err(windows_core::Error::from(e)),
            }
        }
    };
}

#[implement(ISequentialInStream)]
pub struct Rust7zSequentialInStream {
    inner: Mutex<Box<dyn Read>>,
}
impl Rust7zSequentialInStream {
    pub fn new(inner: Mutex<Box<dyn Read>>) -> Self {
        Self {
            inner,
        }
    }

    pub fn into_inner(self) -> Mutex<Box<dyn Read>> {
        self.inner
    }
}
impl ISequentialInStream_Impl for Rust7zSequentialInStream_Impl {
    impl_read!();
}

pub trait ReadSeek : Read + Seek {}
impl<R: Read + Seek> ReadSeek for R {}

#[implement(IInStream)]
pub struct Rust7zInStream {
    inner: Mutex<Box<dyn ReadSeek>>,
}
impl Rust7zInStream {
    pub fn new(inner: Mutex<Box<dyn ReadSeek>>) -> Self {
        Self {
            inner,
        }
    }

    pub fn into_inner(self) -> Mutex<Box<dyn ReadSeek>> {
        self.inner
    }
}
impl ISequentialInStream_Impl for Rust7zInStream_Impl {
    impl_read!();
}
impl IInStream_Impl for Rust7zInStream_Impl {
    fn Seek(&self, offset: i64, seek_origin: u32, new_pos_ptr: *mut u64) -> windows_core::Result<()> {
        const SEEK_SET: u32 = STREAM_SEEK_SET.0;
        const SEEK_CUR: u32 = STREAM_SEEK_CUR.0;
        const SEEK_END: u32 = STREAM_SEEK_END.0;

        let seek_from = match seek_origin {
            SEEK_SET => {
                let Ok(offset_u64) = offset.try_into() else {
                    return Err(windows_core::Error::from(ERROR_INVALID_PARAMETER));
                };
                SeekFrom::Start(offset_u64)
            },
            SEEK_CUR => SeekFrom::Current(offset),
            SEEK_END => SeekFrom::End(offset),
            _ => return Err(windows_core::Error::from(ERROR_INVALID_PARAMETER)),
        };

        let seek_res = {
            let mut guard = self.inner
                .lock().expect("locking mutex failed");
            guard.seek(seek_from)
        };
        let new_pos = seek_res
            .map_err(|e| windows_core::Error::from(e))?;
        if !new_pos_ptr.is_null() {
            unsafe {
                *new_pos_ptr = new_pos;
            }
        }
        Ok(())
    }
}
