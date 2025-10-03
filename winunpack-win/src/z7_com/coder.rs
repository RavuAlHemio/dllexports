use std::ffi::c_void;

use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows_core::{interface, IUnknown, IUnknown_Vtbl, GUID, HRESULT};

use crate::z7_com::PROPID;
use crate::z7_com::stream::{ISequentialInStream, ISequentialOutStream};


#[interface("23170F69-40C1-278A-0000-000400040000")]
pub unsafe trait ICompressProgressInfo : IUnknown {
    fn SetRatioInfo(&self, in_size: *const u64, out_size: *const u64) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400050000")]
pub unsafe trait ICompressCoder : IUnknown {
    fn Code(
        &self,
        in_stream: *mut ISequentialInStream,
        out_stream: *mut ISequentialOutStream,
        in_size: *const u64, out_size: *const u64,
        progress: *mut ICompressProgressInfo,
    ) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400180000")]
pub unsafe trait ICompressCoder2 : IUnknown {
    fn Code(
        &self,
        in_streams: *const *mut ISequentialInStream,
        in_sizes: *const *const u64,
        num_in_streams: u32,
        out_streams: *const *mut ISequentialOutStream,
        out_sizes: *const *const u64,
        num_out_streams: u32,
        progress: *mut ICompressProgressInfo,
    ) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-0004001F0000")]
pub unsafe trait ICompressSetCoderPropertiesOpt : IUnknown {
    fn SetCoderPropertiesOpt(&self, prop_ids: *const PROPID, props: *const PROPVARIANT, num_props: u32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400200000")]
pub unsafe trait ICompressSetCoderProperties : IUnknown {
    fn SetCoderProperties(&self, prop_ids: *const PROPID, props: *const PROPVARIANT, num_props: u32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400220000")]
pub unsafe trait ICompressSetDecoderProperties2 : IUnknown {
    fn SetDecoderProperties2(&self, data: *const u8, size: u32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400230000")]
pub unsafe trait ICompressWriteCoderProperties : IUnknown {
    fn WriteCoderProperties(&self, out_stream: *mut ISequentialOutStream) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400240000")]
pub unsafe trait ICompressGetInStreamProcessedSize : IUnknown {
    fn GetInStreamProcessedSize(&self, value: *mut u64) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400250000")]
pub unsafe trait ICompressSetCoderMt : IUnknown {
    fn SetNumberOfThreads(&self, num_threads: u32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400260000")]
pub unsafe trait ICompressSetFinishMode : IUnknown {
    fn SetFinishMode(&self, finish_mode: u32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400270000")]
pub unsafe trait ICompressGetInStreamProcessedSize2 : IUnknown {
    fn GetInStreamProcessedSize2(&self, stream_index: u32, value: *mut u64) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400280000")]
pub unsafe trait ICompressSetMemLimit : IUnknown {
    fn SetMemLimit(&self, mem_usage: u64) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400290000")]
pub unsafe trait ICompressReadUnusedFromInBuf : IUnknown {
    fn ReadUnusedFromInBuf(&self, data: *mut c_void, size: u32, processed_size: *mut u32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400300000")]
pub unsafe trait ICompressGetSubStreamSize : IUnknown {
    fn GetSubStreamSize(&self, sub_stream: u64, value: *mut u64) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400310000")]
pub unsafe trait ICompressSetInStream : IUnknown {
    fn SetInStream(&self, in_stream: *mut ISequentialInStream) -> HRESULT;
    fn ReleaseInStream(&self) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400320000")]
pub unsafe trait ICompressSetOutStream : IUnknown {
    fn SetOutStream(&self, out_stream: *mut ISequentialInStream) -> HRESULT;
    fn ReleaseOutStream(&self) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400340000")]
pub unsafe trait ICompressSetOutStreamSize : IUnknown {
    fn SetOutStreamSize(&self, out_size: *const u64) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400350000")]
pub unsafe trait ICompressSetBufSize : IUnknown {
    fn SetInBufSize(&self, stream_index: u32, size: u32) -> HRESULT;
    fn SetOutBufSize(&self, stream_index: u32, size: u32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400360000")]
pub unsafe trait ICompressInitEncoder : IUnknown {
    fn InitEncoder(&self) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400370000")]
pub unsafe trait ICompressSetInStream2 : IUnknown {
    fn SetInStream2(&self, stream_index: u32, in_stream: *mut ISequentialInStream) -> HRESULT;
    fn ReleaseInStream2(&self, stream_index: u32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400400000")]
pub unsafe trait ICompressFilter : IUnknown {
    fn Init(&self) -> HRESULT;
    fn Filter(&self, data: *mut u8, size: u32) -> u32;
}

#[interface("23170F69-40C1-278A-0000-000400600000")]
pub unsafe trait ICompressCodecsInfo : IUnknown {
    fn GetNumMethods(&self, num_methods: *mut u32) -> HRESULT;
    fn GetProperty(&self, index: u32, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn CreateDecoder(&self, index: u32, iid: *const GUID, coder: *mut *mut c_void) -> HRESULT;
    fn CreateEncoder(&self, index: u32, iid: *const GUID, coder: *mut *mut c_void) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400610000")]
pub unsafe trait ISetCompressCodecsInfo : IUnknown {
    fn SetCompressCodecsInfo(&self, compress_codecs_info: *mut ICompressCodecsInfo) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400800000")]
pub unsafe trait ICryptoProperties : IUnknown {
    fn SetKey(&self, data: *const u8, size: u32) -> HRESULT;
    fn SetInitVector(&self, data: *const u8, size: u32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-0004008C0000")]
pub unsafe trait ICryptoResetInitVector : IUnknown {
    fn ResetInitVector(&self) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400900000")]
pub unsafe trait ICryptoSetPassword : IUnknown {
    fn CryptoSetPassword(&self) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400A00000")]
pub unsafe trait ICryptoSetCRC : IUnknown {
    fn CryptoSetCRC(&self, crc: u32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000400C00000")]
pub unsafe trait IHasher : IUnknown {
    fn Init(&self) -> ();
    fn Update(&self, data: *const c_void, size: u32);
    fn Final(&self, digest: *mut u8);
    fn GetDigestSize(&self) -> u32;
}

#[interface("23170F69-40C1-278A-0000-000400C10000")]
pub unsafe trait IHashers : IUnknown {
    fn GetNumHashers(&self) -> u32;
    fn GetHasherProp(&self, index: u32, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn CreateHasher(&self, index: u32, hasher: *mut *mut IHasher) -> HRESULT;
}
