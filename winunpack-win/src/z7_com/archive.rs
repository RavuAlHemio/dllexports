use std::ffi::c_void;

use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows_core::{interface, IUnknown, IUnknown_Vtbl, BSTR, HRESULT};

use crate::z7_com::{PROPID, VARTYPE, wchar_t};
use crate::z7_com::progress::{IProgress, IProgress_Impl, IProgress_Vtbl};
use crate::z7_com::stream::{IInStream, ISequentialInStream, ISequentialOutStream};


#[interface("23170F69-40C1-278A-0000-000600100000")]
pub unsafe trait IArchiveOpenCallback : IUnknown {
    fn SetTotal(&self, files: *const u64, bytes: *const u64) -> HRESULT;
    fn SetCompleted(&self, files: *const u64, bytes: *const u64) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000600200000")]
pub unsafe trait IArchiveExtractCallback : IProgress {
    fn GetStream(&self, index: u32, out_stream: *mut *mut ISequentialOutStream, ask_extract_mode: i32) -> HRESULT;
    fn PrepareOperation(&self, ask_extract_mode: i32) -> HRESULT;
    fn SetOperationResult(&self, op_res: i32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000600220000")]
pub unsafe trait IArchiveExtractCallbackMessage2 : IUnknown {
    fn ReportExtractResult(&self, index_type: u32, index: u32, op_res: i32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000600300000")]
pub unsafe trait IArchiveOpenVolumeCallback : IUnknown {
    fn GetProperty(&self, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn GetStream(&self, name: *const wchar_t, in_stream: *mut *mut IInStream) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000600400000")]
pub unsafe trait IInArchiveGetStream : IUnknown {
    fn GetStream(&self, index: u32, stream: *mut *mut ISequentialInStream) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000600500000")]
pub unsafe trait IArchiveOpenSetSubArchiveName : IUnknown {
    fn SetSubArchiveName(&self, name: *const wchar_t) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000600600000")]
pub unsafe trait IInArchive : IUnknown {
    fn Open(&self, stream: *mut IInStream, max_check_start_position: *const u64, open_callback: *mut IArchiveOpenCallback) -> HRESULT;
    fn Close(&self) -> HRESULT;
    fn GetNumberOfItems(&self, num_items: *mut u32) -> HRESULT;
    fn GetProperty(&self, index: u32, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn Extract(&self, indices: *const u32, num_items: u32, test_mode: i32, extractCallback: *mut IArchiveExtractCallback) -> HRESULT;
    fn GetArchiveProperty(&self, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn GetNumberOfProperties(&self, num_props: *mut u32) -> HRESULT;
    fn GetPropertyInfo(&self, index: u32, name: *mut BSTR, prop_id: *mut PROPID, var_type: *mut VARTYPE) -> HRESULT;
    fn GetNumberOfArchiveProperties(&self, num_props: *mut u32) -> HRESULT;
    fn GetArchivePropertyInfo(&self, index: u32, name: *mut BSTR, prop_id: *mut PROPID, var_type: *mut VARTYPE) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000600610000")]
pub unsafe trait IArchiveOpenSeq : IUnknown {
    fn OpenSeq(&self, stream: *mut ISequentialInStream) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000600700000")]
pub unsafe trait IArchiveGetRawProps : IUnknown {
    fn GetParent(&self, index: u32, parent: *mut u32, parent_type: *mut u32) -> HRESULT;
    fn GetRawProp(&self, index: u32, prop_id: PROPID, data: *const *mut c_void, data_size: *mut u32, prop_type: *mut u32) -> HRESULT;
    fn GetNumRawProps(&self, num_props: *mut u32) -> HRESULT;
    fn GetRawPropInfo(&self, index: u32, name: *mut BSTR, prop_id: *mut PROPID) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000600710000")]
pub unsafe trait IArchiveGetRootProps : IUnknown {
    fn GetRootProp(&self, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn GetRootRawProp(&self, prop_id: PROPID, data: *const *mut c_void, data_size: *mut u32, prop_type: *mut u32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000600800000")]
pub unsafe trait IArchiveUpdateCallback : IProgress {
    fn GetUpdateItemInfo(&self, index: u32, new_data: *mut i32, new_props: *mut i32, index_in_archive: *mut u32) -> HRESULT;
    fn GetProperty(&self, index: u32, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn GetStream(&self, index: u32, in_stream: *mut *mut ISequentialInStream) -> HRESULT;
    fn SetOperationResult(&self, operation_result: i32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000600820000")]
pub unsafe trait IArchiveUpdateCallback2 : IArchiveUpdateCallback {
    fn GetVolumeSize(&self, index: u32, size: *mut u64) -> HRESULT;
    fn GetVolumeStream(&self, index: u32, volume_stream: *mut *mut ISequentialOutStream) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000600830000")]
pub unsafe trait IArchiveUpdateCallbackFile : IUnknown {
    fn GetStream2(&self, index: u32, in_stream: *mut *mut ISequentialInStream, notify_op: u32) -> HRESULT;
    fn ReportOperation(&self, index_type: u32, index: u32, notify_op: u32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000600840000")]
pub unsafe trait IArchiveGetDiskProperty : IUnknown {
    fn GetDiskProperty(&self, index: u32, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000600A00000")]
pub unsafe trait IOutArchive : IUnknown {
    fn UpdateItems(&self, out_stream: *mut ISequentialOutStream, num_items: u32, update_callback: *mut IArchiveUpdateCallback) -> HRESULT;
    fn GetFileTimeType(&self, time_type: *mut u32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000600030000")]
pub unsafe trait ISetProperties : IUnknown {
    fn SetProperties(&self, names: *const *const wchar_t, values: *const PROPVARIANT, num_props: u32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000600040000")]
pub unsafe trait IArchiveKeepModeForNextOpen : IUnknown {
    fn KeepModeForNextOpen(&self) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000600050000")]
pub unsafe trait IArchiveAllowTail : IUnknown {
    fn AllowTail(&self, allow_tail: i32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000600090000")]
pub unsafe trait IArchiveRequestMemoryUseCallback : IUnknown {
    fn RequestMemoryUse(
        &self,
        flags: u32, index_type: u32, index: u32, path: *const wchar_t,
        required_size: u64, allowed_size: *mut u64, answer_flags: *mut u32,
    ) -> HRESULT;
}
