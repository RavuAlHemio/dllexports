use std::ffi::c_void;

use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows_core::{Error, GUID, HRESULT, IUnknown, IUnknown_Vtbl};
use winunpack_macros::interface_7zip;

use crate::z7_com::PROPID;
use crate::z7_com::stream::{ISequentialInStream, ISequentialOutStream};


#[interface_7zip(4, 0x04)]
pub unsafe trait ICompressProgressInfo : IUnknown {
    pub fn SetRatioInfo_Raw(&self, in_size: *const u64, out_size: *const u64) -> HRESULT;
}

pub trait ICompressProgressInfo_Ext {
    fn SetRatioInfo(&self, in_size: u64, out_size: u64) -> Result<(), Error>;
}
impl<T: ICompressProgressInfo_Impl> ICompressProgressInfo_Ext for T {
    fn SetRatioInfo(&self, in_size: u64, out_size: u64) -> Result<(), Error> {
        unsafe {
            self.SetRatioInfo_Raw(&in_size, &out_size)
                .ok()
        }
    }
}

#[interface_7zip(4, 0x05)]
pub unsafe trait ICompressCoder : IUnknown {
    pub fn Code_Raw(
        &self,
        in_stream: ISequentialInStream,
        out_stream: ISequentialOutStream,
        in_size: *const u64, out_size: *const u64,
        progress: ICompressProgressInfo,
    ) -> HRESULT;
}

pub trait ICompressCoder_Ext {
    fn Code(
        &self,
        in_stream: ISequentialInStream,
        out_stream: ISequentialOutStream,
        in_size: u64, out_size: u64,
        progress: ICompressProgressInfo,
    ) -> Result<(), Error>;
}
impl<T: ICompressCoder_Impl> ICompressCoder_Ext for T {
    fn Code(
        &self,
        in_stream: ISequentialInStream,
        out_stream: ISequentialOutStream,
        in_size: u64, out_size: u64,
        progress: ICompressProgressInfo,
    ) -> Result<(), Error> {
        unsafe {
            self.Code_Raw(
                in_stream,
                out_stream,
                &in_size, &out_size,
                progress,
            )
                .ok()
        }
    }
}

#[interface_7zip(4, 0x18)]
pub unsafe trait ICompressCoder2 : IUnknown {
    pub fn Code_Raw(
        &self,
        in_streams: *const ISequentialInStream,
        in_sizes: *const *const u64,
        num_in_streams: u32,
        out_streams: *const ISequentialOutStream,
        out_sizes: *const *const u64,
        num_out_streams: u32,
        progress: ICompressProgressInfo,
    ) -> HRESULT;
}

pub trait ICompressCoder2_Ext {
    fn Code(
        &self,
        in_streams_and_sizes: &[(ISequentialInStream, u64)],
        out_streams_and_sizes: &[(ISequentialOutStream, u64)],
        progress: ICompressProgressInfo,
    ) -> Result<(), Error>;
}
impl<T: ICompressCoder2_Impl> ICompressCoder2_Ext for T {
    fn Code(
        &self,
        in_streams_and_sizes: &[(ISequentialInStream, u64)],
        out_streams_and_sizes: &[(ISequentialOutStream, u64)],
        progress: ICompressProgressInfo,
    ) -> Result<(), Error> {
        let num_in_streams = in_streams_and_sizes.len();
        let num_in_streams_u32: u32 = num_in_streams.try_into().unwrap();
        let num_out_streams = out_streams_and_sizes.len();
        let num_out_streams_u32: u32 = num_out_streams.try_into().unwrap();

        let mut in_streams = Vec::with_capacity(num_in_streams);
        let mut in_sizes = Vec::with_capacity(num_in_streams);
        for (in_stream, in_size) in in_streams_and_sizes {
            in_streams.push(in_stream.clone());
            in_sizes.push(*in_size);
        }

        let mut in_size_ptrs: Vec<*const u64> = Vec::with_capacity(num_in_streams);
        for in_size in &in_sizes {
            in_size_ptrs.push(in_size);
        }

        let mut out_streams = Vec::with_capacity(num_out_streams);
        let mut out_sizes = Vec::with_capacity(num_out_streams);
        for (out_stream, out_size) in out_streams_and_sizes {
            out_streams.push(out_stream.clone());
            out_sizes.push(*out_size);
        }

        let mut out_size_ptrs: Vec<*const u64> = Vec::with_capacity(num_in_streams);
        for out_size in &out_sizes {
            out_size_ptrs.push(out_size);
        }

        unsafe {
            self.Code_Raw(
                in_streams.as_ptr(),
                in_size_ptrs.as_ptr(),
                num_in_streams_u32,
                out_streams.as_ptr(),
                out_size_ptrs.as_ptr(),
                num_out_streams_u32,
                progress,
            )
                .ok()
        }
    }
}

#[interface_7zip(4, 0x1F)]
pub unsafe trait ICompressSetCoderPropertiesOpt : IUnknown {
    pub fn SetCoderPropertiesOpt_Raw(&self, prop_ids: *const PROPID, props: *const PROPVARIANT, num_props: u32) -> HRESULT;
}

pub trait ICompressSetCoderPropertiesOpt_Ext {
    fn SetCoderPropertiesOpt(&self, props: &[(PROPID, PROPVARIANT)]) -> Result<(), Error>;
}
impl<T: ICompressSetCoderPropertiesOpt_Impl> ICompressSetCoderPropertiesOpt_Ext for T {
    fn SetCoderPropertiesOpt(&self, props: &[(PROPID, PROPVARIANT)]) -> Result<(), Error> {
        let num_props = props.len();
        let num_props_u32: u32 = num_props.try_into().unwrap();

        let mut prop_ids: Vec<PROPID> = Vec::with_capacity(num_props);
        let mut prop_vals: Vec<PROPVARIANT> = Vec::with_capacity(num_props);
        for (pid, pval) in props {
            prop_ids.push(*pid);
            prop_vals.push(pval.clone());
        }

        unsafe {
            self.SetCoderPropertiesOpt_Raw(prop_ids.as_ptr(), prop_vals.as_ptr(), num_props_u32)
                .ok()
        }
    }
}

#[interface_7zip(4, 0x20)]
pub unsafe trait ICompressSetCoderProperties : IUnknown {
    fn SetCoderProperties_Raw(&self, prop_ids: *const PROPID, props: *const PROPVARIANT, num_props: u32) -> HRESULT;
}

pub trait ICompressSetCoderProperties_Ext {
    fn SetCoderProperties(&self, props: &[(PROPID, PROPVARIANT)]) -> Result<(), Error>;
}
impl<T: ICompressSetCoderProperties_Impl> ICompressSetCoderProperties_Ext for T {
    fn SetCoderProperties(&self, props: &[(PROPID, PROPVARIANT)]) -> Result<(), Error> {
        let num_props = props.len();
        let num_props_u32: u32 = num_props.try_into().unwrap();

        let mut prop_ids: Vec<PROPID> = Vec::with_capacity(num_props);
        let mut prop_vals: Vec<PROPVARIANT> = Vec::with_capacity(num_props);
        for (pid, pval) in props {
            prop_ids.push(*pid);
            prop_vals.push(pval.clone());
        }

        unsafe {
            self.SetCoderProperties_Raw(prop_ids.as_ptr(), prop_vals.as_ptr(), num_props_u32)
                .ok()
        }
    }
}

#[interface_7zip(4, 0x22)]
pub unsafe trait ICompressSetDecoderProperties2 : IUnknown {
    fn SetDecoderProperties2_Raw(&self, data: *const u8, size: u32) -> HRESULT;
}

pub trait ICompressSetDecoderProperties2_Ext {
    fn SetDecoderProperties2(&self, data: &[u8]) -> Result<(), Error>;
}
impl<T: ICompressSetDecoderProperties2_Impl> ICompressSetDecoderProperties2_Ext for T {
    fn SetDecoderProperties2(&self, data: &[u8]) -> Result<(), Error> {
        let size: u32 = data.len().try_into().unwrap();

        unsafe {
            self.SetDecoderProperties2_Raw(data.as_ptr(), size)
                .ok()
        }
    }
}

#[interface_7zip(4, 0x23)]
pub unsafe trait ICompressWriteCoderProperties : IUnknown {
    fn WriteCoderProperties_Raw(&self, out_stream: ISequentialOutStream) -> HRESULT;
}

pub trait ICompressWriteCoderProperties_Ext {
    fn WriteCoderProperties(&self, out_stream: ISequentialOutStream) -> Result<(), Error>;
}
impl<T: ICompressWriteCoderProperties_Impl> ICompressWriteCoderProperties_Ext for T {
    fn WriteCoderProperties(&self, out_stream: ISequentialOutStream) -> Result<(), Error> {
        unsafe {
            self.WriteCoderProperties_Raw(out_stream)
                .ok()
        }
    }
}

#[interface_7zip(4, 0x24)]
pub unsafe trait ICompressGetInStreamProcessedSize : IUnknown {
    fn GetInStreamProcessedSize_Raw(&self, value: *mut u64) -> HRESULT;
}

pub trait ICompressGetInStreamProcessedSize_Ext {
    fn GetInStreamProcessedSize(&self) -> Result<u64, Error>;
}
impl<T: ICompressGetInStreamProcessedSize_Impl> ICompressGetInStreamProcessedSize_Ext for T {
    fn GetInStreamProcessedSize(&self) -> Result<u64, Error> {
        let mut value = 0;
        unsafe {
            self.GetInStreamProcessedSize_Raw(&mut value)
                .map(|| value)
        }
    }
}

#[interface_7zip(4, 0x25)]
pub unsafe trait ICompressSetCoderMt : IUnknown {
    fn SetNumberOfThreads_Raw(&self, num_threads: u32) -> HRESULT;
}

pub trait ICompressSetCoderMt_Ext {
    fn SetNumberOfThreads(&self, num_threads: u32) -> Result<(), Error>;
}
impl<T: ICompressSetCoderMt_Impl> ICompressSetCoderMt_Ext for T {
    fn SetNumberOfThreads(&self, num_threads: u32) -> Result<(), Error> {
        unsafe {
            self.SetNumberOfThreads_Raw(num_threads)
                .ok()
        }
    }
}

#[interface_7zip(4, 0x26)]
pub unsafe trait ICompressSetFinishMode : IUnknown {
    fn SetFinishMode_Raw(&self, finish_mode: u32) -> HRESULT;
}

pub trait ICompressSetFinishMode_Ext {
    fn SetFinishMode(&self, finish_mode: u32) -> Result<(), Error>;
}
impl<T: ICompressSetFinishMode_Impl> ICompressSetFinishMode_Ext for T {
    fn SetFinishMode(&self, finish_mode: u32) -> Result<(), Error> {
        unsafe {
            self.SetFinishMode_Raw(finish_mode)
                .ok()
        }
    }
}

#[interface_7zip(4, 0x27)]
pub unsafe trait ICompressGetInStreamProcessedSize2 : IUnknown {
    fn GetInStreamProcessedSize2_Raw(&self, stream_index: u32, value: *mut u64) -> HRESULT;
}

pub trait ICompressGetInStreamProcessedSize2_Ext {
    fn GetInStreamProcessedSize2(&self, stream_index: u32) -> Result<u64, Error>;
}
impl<T: ICompressGetInStreamProcessedSize2_Impl> ICompressGetInStreamProcessedSize2_Ext for T {
    fn GetInStreamProcessedSize2(&self, stream_index: u32) -> Result<u64, Error> {
        let mut value = 0;
        unsafe {
            self.GetInStreamProcessedSize2_Raw(stream_index, &mut value)
                .map(|| value)
        }
    }
}

#[interface_7zip(4, 0x28)]
pub unsafe trait ICompressSetMemLimit : IUnknown {
    fn SetMemLimit_Raw(&self, mem_usage: u64) -> HRESULT;
}

pub trait ICompressSetMemLimit_Ext {
    fn SetMemLimit(&self, mem_usage: u64) -> Result<(), Error>;
}
impl<T: ICompressSetMemLimit_Impl> ICompressSetMemLimit_Ext for T {
    fn SetMemLimit(&self, mem_usage: u64) -> Result<(), Error> {
        unsafe {
            self.SetMemLimit_Raw(mem_usage)
                .ok()
        }
    }
}

#[interface_7zip(4, 0x29)]
pub unsafe trait ICompressReadUnusedFromInBuf : IUnknown {
    fn ReadUnusedFromInBuf_Raw(&self, data: *mut u8, size: u32, processed_size: *mut u32) -> HRESULT;
}

pub trait ICompressReadUnusedFromInBuf_Ext {
    fn ReadUnusedFromInBuf(&self, data: &mut [u8]) -> Result<u32, Error>;
}
impl<T: ICompressReadUnusedFromInBuf_Impl> ICompressReadUnusedFromInBuf_Ext for T {
    fn ReadUnusedFromInBuf(&self, data: &mut [u8]) -> Result<u32, Error> {
        let size: u32 = data.len().try_into().unwrap();
        let mut processed_size = 0;
        unsafe {
            self.ReadUnusedFromInBuf_Raw(data.as_mut_ptr(), size, &mut processed_size)
                .map(|| processed_size)
        }
    }
}

#[interface_7zip(4, 0x30)]
pub unsafe trait ICompressGetSubStreamSize : IUnknown {
    fn GetSubStreamSize_Raw(&self, sub_stream: u64, value: *mut u64) -> HRESULT;
}

pub trait ICompressGetSubStreamSize_Ext {
    fn GetSubStreamSize(&self, sub_stream: u64) -> Result<u64, Error>;
}
impl<T: ICompressGetSubStreamSize_Impl> ICompressGetSubStreamSize_Ext for T {
    fn GetSubStreamSize(&self, sub_stream: u64) -> Result<u64, Error> {
        let mut value = 0;
        unsafe {
            self.GetSubStreamSize_Raw(sub_stream, &mut value)
                .map(|| value)
        }
    }
}

#[interface_7zip(4, 0x31)]
pub unsafe trait ICompressSetInStream : IUnknown {
    fn SetInStream_Raw(&self, in_stream: ISequentialInStream) -> HRESULT;
    fn ReleaseInStream_Raw(&self) -> HRESULT;
}

pub trait ICompressSetInStream_Ext {
    fn SetInStream(&self, in_stream: ISequentialInStream) -> Result<(), Error>;
    fn ReleaseInStream(&self) -> Result<(), Error>;
}
impl<T: ICompressSetInStream_Impl> ICompressSetInStream_Ext for T {
    fn SetInStream(&self, in_stream: ISequentialInStream) -> Result<(), Error> {
        unsafe {
            self.SetInStream_Raw(in_stream)
                .ok()
        }
    }

    fn ReleaseInStream(&self) -> Result<(), Error> {
        unsafe {
            self.ReleaseInStream_Raw()
                .ok()
        }
    }
}

#[interface_7zip(4, 0x32)]
pub unsafe trait ICompressSetOutStream : IUnknown {
    fn SetOutStream_Raw(&self, out_stream: ISequentialOutStream) -> HRESULT;
    fn ReleaseOutStream_Raw(&self) -> HRESULT;
}

pub trait ICompressSetOutStream_Ext {
    fn SetOutStream(&self, out_stream: ISequentialOutStream) -> Result<(), Error>;
    fn ReleaseOutStream(&self) -> Result<(), Error>;
}
impl<T: ICompressSetOutStream_Impl> ICompressSetOutStream_Ext for T {
    fn SetOutStream(&self, out_stream: ISequentialOutStream) -> Result<(), Error> {
        unsafe {
            self.SetOutStream_Raw(out_stream)
                .ok()
        }
    }

    fn ReleaseOutStream(&self) -> Result<(), Error> {
        unsafe {
            self.ReleaseOutStream_Raw()
                .ok()
        }
    }
}

#[interface_7zip(4, 0x34)]
pub unsafe trait ICompressSetOutStreamSize : IUnknown {
    fn SetOutStreamSize_Raw(&self, out_size: *const u64) -> HRESULT;
}

pub trait ICompressSetOutStreamSize_Ext {
    fn SetOutStreamSize(&self) -> Result<u64, Error>;
}
impl<T: ICompressSetOutStreamSize_Impl> ICompressSetOutStreamSize_Ext for T {
    fn SetOutStreamSize(&self) -> Result<u64, Error> {
        let mut out_size = 0;
        unsafe {
            self.SetOutStreamSize_Raw(&mut out_size)
                .map(|| out_size)
        }
    }
}

#[interface_7zip(4, 0x35)]
pub unsafe trait ICompressSetBufSize : IUnknown {
    fn SetInBufSize_Raw(&self, stream_index: u32, size: u32) -> HRESULT;
    fn SetOutBufSize_Raw(&self, stream_index: u32, size: u32) -> HRESULT;
}

pub trait ICompressSetBufSize_Ext {
    fn SetInBufSize(&self, stream_index: u32, size: u32) -> Result<(), Error>;
    fn SetOutBufSize(&self, stream_index: u32, size: u32) -> Result<(), Error>;
}
impl<T: ICompressSetBufSize_Impl> ICompressSetBufSize_Ext for T {
    fn SetInBufSize(&self, stream_index: u32, size: u32) -> Result<(), Error> {
        unsafe {
            self.SetInBufSize_Raw(stream_index, size)
                .ok()
        }
    }

    fn SetOutBufSize(&self, stream_index: u32, size: u32) -> Result<(), Error> {
        unsafe {
            self.SetOutBufSize_Raw(stream_index, size)
                .ok()
        }
    }
}

#[interface_7zip(4, 0x36)]
pub unsafe trait ICompressInitEncoder : IUnknown {
    fn InitEncoder_Raw(&self) -> HRESULT;
}

pub trait ICompressInitEncoder_Ext {
    fn InitEncoder(&self) -> Result<(), Error>;
}
impl<T: ICompressInitEncoder_Impl> ICompressInitEncoder_Ext for T {
    fn InitEncoder(&self) -> Result<(), Error> {
        unsafe {
            self.InitEncoder_Raw()
                .ok()
        }
    }
}

#[interface_7zip(4, 0x37)]
pub unsafe trait ICompressSetInStream2 : IUnknown {
    fn SetInStream2(&self, stream_index: u32, in_stream: ISequentialInStream) -> HRESULT;
    fn ReleaseInStream2(&self, stream_index: u32) -> HRESULT;
}

#[interface_7zip(4, 0x40)]
pub unsafe trait ICompressFilter : IUnknown {
    fn Init(&self) -> HRESULT;
    fn Filter(&self, data: *mut u8, size: u32) -> u32;
}

#[interface_7zip(4, 0x60)]
pub unsafe trait ICompressCodecsInfo : IUnknown {
    fn GetNumMethods(&self, num_methods: *mut u32) -> HRESULT;
    fn GetProperty(&self, index: u32, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn CreateDecoder(&self, index: u32, iid: *const GUID, coder: *mut *mut c_void) -> HRESULT;
    fn CreateEncoder(&self, index: u32, iid: *const GUID, coder: *mut *mut c_void) -> HRESULT;
}

#[interface_7zip(4, 0x61)]
pub unsafe trait ISetCompressCodecsInfo : IUnknown {
    fn SetCompressCodecsInfo(&self, compress_codecs_info: ICompressCodecsInfo) -> HRESULT;
}

#[interface_7zip(4, 0x80)]
pub unsafe trait ICryptoProperties : IUnknown {
    fn SetKey(&self, data: *const u8, size: u32) -> HRESULT;
    fn SetInitVector(&self, data: *const u8, size: u32) -> HRESULT;
}

#[interface_7zip(4, 0x8C)]
pub unsafe trait ICryptoResetInitVector : IUnknown {
    fn ResetInitVector(&self) -> HRESULT;
}

#[interface_7zip(4, 0x90)]
pub unsafe trait ICryptoSetPassword : IUnknown {
    fn CryptoSetPassword(&self) -> HRESULT;
}

#[interface_7zip(4, 0xA0)]
pub unsafe trait ICryptoSetCRC : IUnknown {
    fn CryptoSetCRC(&self, crc: u32) -> HRESULT;
}

#[interface_7zip(4, 0xC0)]
pub unsafe trait IHasher : IUnknown {
    fn Init(&self) -> ();
    fn Update(&self, data: *const c_void, size: u32);
    fn Final(&self, digest: *mut u8);
    fn GetDigestSize(&self) -> u32;
}

#[interface_7zip(4, 0xC1)]
pub unsafe trait IHashers : IUnknown {
    fn GetNumHashers(&self) -> u32;
    fn GetHasherProp(&self, index: u32, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn CreateHasher(&self, index: u32, hasher: *mut *mut IHasher) -> HRESULT;
}
