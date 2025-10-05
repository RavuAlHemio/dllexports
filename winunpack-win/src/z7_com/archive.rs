use std::ffi::c_void;

use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows_core::{Error, IUnknown, IUnknown_Vtbl, BSTR, HRESULT, Type};
use winunpack_macros::interface_7zip;

use crate::z7_com::{PROPID, VARTYPE, wchar_t};
use crate::z7_com::progress::{IProgress, IProgress_Impl, IProgress_Vtbl};
use crate::z7_com::stream::{IInStream, ISequentialInStream, ISequentialOutStream};


#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RawPropertyInfo {
    pub name: BSTR,
    pub prop_id: PROPID,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PropertyInfo {
    pub raw: RawPropertyInfo,
    pub var_type: VARTYPE,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ParentInfo {
    pub parent: u32,
    pub parent_type: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RawPropertyValue {
    pub data: *const c_void,
    pub data_size: u32,
    pub prop_type: u32,
}

#[interface_7zip(6, 0x10)]
pub unsafe trait IArchiveOpenCallback : IUnknown {
    pub fn SetTotal_Raw(&self, files: *const u64, bytes: *const u64) -> HRESULT;
    pub fn SetCompleted_Raw(&self, files: *const u64, bytes: *const u64) -> HRESULT;
}

pub trait IArchiveOpenCallback_Ext {
    fn SetTotal(&self, files: u64, bytes: u64) -> Result<(), Error>;
}
impl<T: IArchiveOpenCallback_Impl> IArchiveOpenCallback_Ext for T {
    fn SetTotal(&self, files: u64, bytes: u64) -> Result<(), Error> {
        unsafe {
            self.SetTotal_Raw(&files, &bytes)
                .ok()
        }
    }
}

#[interface_7zip(6, 0x20)]
pub unsafe trait IArchiveExtractCallback : IProgress {
    pub fn GetStream_Raw(&self, index: u32, out_stream: *mut *mut c_void, ask_extract_mode: i32) -> HRESULT;
    pub fn PrepareOperation_Raw(&self, ask_extract_mode: i32) -> HRESULT;
    pub fn SetOperationResult_Raw(&self, op_res: i32) -> HRESULT;
}

pub trait IArchiveExtractCallback_Ext {
    fn GetStream(&self, index: u32, ask_extract_mode: i32) -> Result<ISequentialOutStream, Error>;
    fn PrepareOperation(&self, ask_extract_mode: i32) -> Result<(), Error>;
    fn SetOperationResult(&self, op_res: i32) -> Result<(), Error>;
}
impl<T: IArchiveExtractCallback_Impl> IArchiveExtractCallback_Ext for T {
    fn GetStream(&self, index: u32, ask_extract_mode: i32) -> Result<ISequentialOutStream, Error> {
        let mut out_stream: *mut c_void = std::ptr::null_mut();
        unsafe {
            self.GetStream_Raw(index, &mut out_stream, ask_extract_mode)
                .and_then(|| Type::from_abi(out_stream))
        }
    }

    fn PrepareOperation(&self, ask_extract_mode: i32) -> Result<(), Error> {
        unsafe {
            self.PrepareOperation_Raw(ask_extract_mode)
                .ok()
        }
    }

    fn SetOperationResult(&self, op_res: i32) -> Result<(), Error> {
        unsafe {
            self.SetOperationResult_Raw(op_res)
                .ok()
        }
    }
}

#[interface_7zip(6, 0x22)]
pub unsafe trait IArchiveExtractCallbackMessage2 : IUnknown {
    pub fn ReportExtractResult_Raw(&self, index_type: u32, index: u32, op_res: i32) -> HRESULT;
}

pub trait IArchiveExtractCallbackMessage2_Ext {
    fn ReportExtractResult(&self, index_type: u32, index: u32, op_res: i32) -> Result<(), Error>;
}
impl<T: IArchiveExtractCallbackMessage2_Impl> IArchiveExtractCallbackMessage2_Ext for T {
    fn ReportExtractResult(&self, index_type: u32, index: u32, op_res: i32) -> Result<(), Error> {
        unsafe {
            self.ReportExtractResult_Raw(index_type, index, op_res)
                .ok()
        }
    }
}

#[interface_7zip(6, 0x30)]
pub unsafe trait IArchiveOpenVolumeCallback : IUnknown {
    pub fn GetProperty_Raw(&self, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    pub fn GetStream_Raw(&self, name: *const wchar_t, in_stream: *mut *mut c_void) -> HRESULT;
}

pub trait IArchiveOpenVolumeCallback_Ext {
    fn GetProperty(&self, prop_id: PROPID) -> Result<PROPVARIANT, Error>;
    fn GetStream(&self, name: *const wchar_t) -> Result<IInStream, Error>;
}
impl<T: IArchiveOpenVolumeCallback_Impl> IArchiveOpenVolumeCallback_Ext for T {
    fn GetProperty(&self, prop_id: PROPID) -> Result<PROPVARIANT, Error> {
        let mut value = PROPVARIANT::default();
        unsafe {
            self.GetProperty_Raw(prop_id, &mut value)
                .map(|| value)
        }
    }

    fn GetStream(&self, name: *const wchar_t) -> Result<IInStream, Error> {
        let mut in_stream: *mut c_void = std::ptr::null_mut();
        unsafe {
            self.GetStream_Raw(name, &mut in_stream)
                .and_then(|| Type::from_abi(in_stream))
        }
    }
}

#[interface_7zip(6, 0x40)]
pub unsafe trait IInArchiveGetStream : IUnknown {
    pub fn GetStream_Raw(&self, index: u32, stream: *mut *mut c_void) -> HRESULT;
}

pub trait IInArchiveGetStream_Ext {
    fn GetStream(&self, index: u32) -> Result<ISequentialInStream, Error>;
}
impl<T: IInArchiveGetStream_Impl> IInArchiveGetStream_Ext for T {
    fn GetStream(&self, index: u32) -> Result<ISequentialInStream, Error> {
        let mut stream: *mut c_void = std::ptr::null_mut();
        unsafe {
            self.GetStream_Raw(index, &mut stream)
                .and_then(|| Type::from_abi(stream))
        }
    }
}

#[interface_7zip(6, 0x50)]
pub unsafe trait IArchiveOpenSetSubArchiveName : IUnknown {
    pub fn SetSubArchiveName_Raw(&self, name: *const wchar_t) -> HRESULT;
}

pub trait IArchiveOpenSetSubArchiveName_Ext {
    fn SetSubArchiveName(&self, name: *const wchar_t) -> Result<(), Error>;
}
impl<T: IArchiveOpenSetSubArchiveName_Impl> IArchiveOpenSetSubArchiveName_Ext for T {
    fn SetSubArchiveName(&self, name: *const wchar_t) -> Result<(), Error> {
        unsafe {
            self.SetSubArchiveName_Raw(name)
                .ok()
        }
    }
}

#[interface_7zip(6, 0x60)]
pub unsafe trait IInArchive : IUnknown {
    pub fn Open_Raw(&self, stream: IInStream, max_check_start_position: *const u64, open_callback: IArchiveOpenCallback) -> HRESULT;
    pub fn Close_Raw(&self) -> HRESULT;
    pub fn GetNumberOfItems_Raw(&self, num_items: *mut u32) -> HRESULT;
    pub fn GetProperty_Raw(&self, index: u32, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    pub fn Extract_Raw(&self, indices: *const u32, num_items: u32, test_mode: i32, extract_callback: IArchiveExtractCallback) -> HRESULT;
    pub fn GetArchiveProperty_Raw(&self, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    pub fn GetNumberOfProperties_Raw(&self, num_props: *mut u32) -> HRESULT;
    pub fn GetPropertyInfo_Raw(&self, index: u32, name: *mut BSTR, prop_id: *mut PROPID, var_type: *mut VARTYPE) -> HRESULT;
    pub fn GetNumberOfArchiveProperties_Raw(&self, num_props: *mut u32) -> HRESULT;
    pub fn GetArchivePropertyInfo_Raw(&self, index: u32, name: *mut BSTR, prop_id: *mut PROPID, var_type: *mut VARTYPE) -> HRESULT;
}

pub trait IInArchive_Ext {
    fn Open(&self, stream: IInStream, max_check_start_position: Option<u64>, open_callback: IArchiveOpenCallback) -> Result<(), Error>;
    fn Close(&self) -> Result<(), Error>;
    fn GetNumberOfItems(&self) -> Result<u32, Error>;
    fn GetProperty(&self, index: u32, prop_id: PROPID) -> Result<PROPVARIANT, Error>;
    fn Extract(&self, indices: &[u32], test_mode: i32, extract_callback: IArchiveExtractCallback) -> Result<(), Error>;
    fn GetArchiveProperty(&self, prop_id: PROPID) -> Result<PROPVARIANT, Error>;
    fn GetNumberOfProperties(&self) -> Result<u32, Error>;
    fn GetPropertyInfo(&self, index: u32) -> Result<PropertyInfo, Error>;
    fn GetNumberOfArchiveProperties(&self) -> Result<u32, Error>;
    fn GetArchivePropertyInfo(&self, index: u32) -> Result<PropertyInfo, Error>;
}
impl<T: IInArchive_Impl> IInArchive_Ext for T {
    fn Open(&self, stream: IInStream, max_check_start_position: Option<u64>, open_callback: IArchiveOpenCallback) -> Result<(), Error> {
        let mcsp;
        let mcsp_ptr = if let Some(m) = max_check_start_position {
            mcsp = m;
            &mcsp
        } else {
            std::ptr::null()
        };

        unsafe {
            self.Open_Raw(stream, mcsp_ptr, open_callback)
                .ok()
        }
    }

    fn Close(&self) -> Result<(), Error> {
        unsafe {
            self.Close_Raw()
                .ok()
        }
    }

    fn GetNumberOfItems(&self) -> Result<u32, Error> {
        let mut num_items = 0;
        unsafe {
            self.GetNumberOfItems_Raw(&mut num_items)
                .map(|| num_items)
        }
    }

    fn GetProperty(&self, index: u32, prop_id: PROPID) -> Result<PROPVARIANT, Error> {
        let mut value = PROPVARIANT::default();
        unsafe {
            self.GetProperty_Raw(index, prop_id, &mut value)
                .map(|| value)
        }
    }

    fn Extract(&self, indices: &[u32], test_mode: i32, extract_callback: IArchiveExtractCallback) -> Result<(), Error> {
        unsafe {
            self.Extract_Raw(
                indices.as_ptr(),
                indices.len().try_into().unwrap(),
                test_mode,
                extract_callback,
            )
                .ok()
        }
    }

    fn GetArchiveProperty(&self, prop_id: PROPID) -> Result<PROPVARIANT, Error> {
        let mut value = PROPVARIANT::default();
        unsafe {
            self.GetArchiveProperty_Raw(prop_id, &mut value)
                .map(|| value)
        }
    }

    fn GetNumberOfProperties(&self) -> Result<u32, Error> {
        let mut num_props = 0;
        unsafe {
            self.GetNumberOfProperties_Raw(&mut num_props)
                .map(|| num_props)
        }
    }

    fn GetPropertyInfo(&self, index: u32) -> Result<PropertyInfo, Error> {
        let mut prop_info = PropertyInfo::default();
        unsafe {
            self.GetPropertyInfo_Raw(
                index,
                &mut prop_info.raw.name,
                &mut prop_info.raw.prop_id,
                &mut prop_info.var_type,
            )
                .map(|| prop_info)
        }
    }

    fn GetNumberOfArchiveProperties(&self) -> Result<u32, Error> {
        let mut num_props = 0;
        unsafe {
            self.GetNumberOfArchiveProperties_Raw(&mut num_props)
                .map(|| num_props)
        }
    }

    fn GetArchivePropertyInfo(&self, index: u32) -> Result<PropertyInfo, Error> {
        let mut prop_info = PropertyInfo::default();
        unsafe {
            self.GetArchivePropertyInfo_Raw(
                index,
                &mut prop_info.raw.name,
                &mut prop_info.raw.prop_id,
                &mut prop_info.var_type,
            )
                .map(|| prop_info)
        }
    }
}

#[interface_7zip(6, 0x61)]
pub unsafe trait IArchiveOpenSeq : IUnknown {
    pub fn OpenSeq_Raw(&self, stream: ISequentialInStream) -> HRESULT;
}

pub trait IArchiveOpenSeq_Ext {
    fn OpenSeq(&self, stream: ISequentialInStream) -> Result<(), Error>;
}
impl<T: IArchiveOpenSeq_Impl> IArchiveOpenSeq_Ext for T {
    fn OpenSeq(&self, stream: ISequentialInStream) -> Result<(), Error> {
        unsafe {
            self.OpenSeq_Raw(stream)
                .ok()
        }
    }
}

#[interface_7zip(6, 0x70)]
pub unsafe trait IArchiveGetRawProps : IUnknown {
    pub fn GetParent_Raw(&self, index: u32, parent: *mut u32, parent_type: *mut u32) -> HRESULT;
    pub fn GetRawProp_Raw(&self, index: u32, prop_id: PROPID, data: *mut *const c_void, data_size: *mut u32, prop_type: *mut u32) -> HRESULT;
    pub fn GetNumRawProps_Raw(&self, num_props: *mut u32) -> HRESULT;
    pub fn GetRawPropInfo_Raw(&self, index: u32, name: *mut BSTR, prop_id: *mut PROPID) -> HRESULT;
}

pub trait IArchiveGetRawProps_Ext {
    fn GetParent(&self, index: u32) -> Result<ParentInfo, Error>;
    fn GetRawProp(&self, index: u32, prop_id: PROPID) -> Result<RawPropertyValue, Error>;
    fn GetNumRawProps(&self) -> Result<u32, Error>;
    fn GetRawPropInfo(&self, index: u32) -> Result<RawPropertyInfo, Error>;
}
impl<T: IArchiveGetRawProps_Impl> IArchiveGetRawProps_Ext for T {
    fn GetParent(&self, index: u32) -> Result<ParentInfo, Error> {
        let mut parent_info = ParentInfo::default();
        unsafe {
            self.GetParent_Raw(index, &mut parent_info.parent, &mut parent_info.parent_type)
                .map(|| parent_info)
        }
    }

    fn GetRawProp(&self, index: u32, prop_id: PROPID) -> Result<RawPropertyValue, Error> {
        let mut prop_value = RawPropertyValue::default();
        unsafe {
            self.GetRawProp_Raw(
                index,
                prop_id,
                &mut prop_value.data,
                &mut prop_value.data_size,
                &mut prop_value.prop_type,
            )
                .map(|| prop_value)
        }
    }

    fn GetNumRawProps(&self) -> Result<u32, Error> {
        let mut num_props = 0;
        unsafe {
            self.GetNumRawProps_Raw(&mut num_props)
                .map(|| num_props)
        }
    }

    fn GetRawPropInfo(&self, index: u32) -> Result<RawPropertyInfo, Error> {
        let mut prop_info = RawPropertyInfo::default();
        unsafe {
            self.GetRawPropInfo_Raw(index, &mut prop_info.name, &mut prop_info.prop_id)
                .map(|| prop_info)
        }
    }
}

#[interface_7zip(6, 0x71)]
pub unsafe trait IArchiveGetRootProps : IUnknown {
    fn GetRootProp_Raw(&self, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn GetRootRawProp_Raw(&self, prop_id: PROPID, data: *mut *const c_void, data_size: *mut u32, prop_type: *mut u32) -> HRESULT;
}

pub trait IArchiveGetRootProps_Ext {
    fn GetRootProp(&self, prop_id: PROPID) -> Result<PROPVARIANT, Error>;
    fn GetRootRawProp(&self, prop_id: PROPID) -> Result<RawPropertyValue, Error>;
}
impl<T: IArchiveGetRootProps_Impl> IArchiveGetRootProps_Ext for T {
    fn GetRootProp(&self, prop_id: PROPID) -> Result<PROPVARIANT, Error> {
        let mut value = PROPVARIANT::default();
        unsafe {
            self.GetRootProp_Raw(prop_id, &mut value)
                .map(|| value)
        }
    }

    fn GetRootRawProp(&self, prop_id: PROPID) -> Result<RawPropertyValue, Error> {
        let mut value = RawPropertyValue::default();
        unsafe {
            self.GetRootRawProp_Raw(
                prop_id,
                &mut value.data,
                &mut value.data_size,
                &mut value.prop_type,
            )
                .map(|| value)
        }
    }
}

#[interface_7zip(6, 0x80)]
pub unsafe trait IArchiveUpdateCallback : IProgress {
    fn GetUpdateItemInfo_Raw(&self, index: u32, new_data: *mut i32, new_props: *mut i32, index_in_archive: *mut u32) -> HRESULT;
    fn GetProperty_Raw(&self, index: u32, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn GetStream_Raw(&self, index: u32, in_stream: *mut *mut c_void /*ISequentialInStream*/) -> HRESULT;
    fn SetOperationResult_Raw(&self, operation_result: i32) -> HRESULT;
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UpdateData {
    pub new_data: i32,
    pub new_props: i32,
    pub index_in_archive: u32,
}

pub trait IArchiveUpdateCallback_Ext {
    fn GetUpdateItemInfo(&self, index: u32) -> Result<UpdateData, Error>;
    fn GetProperty(&self, index: u32, prop_id: PROPID) -> Result<PROPVARIANT, Error>;
    fn GetStream(&self, index: u32) -> Result<ISequentialInStream, Error>;
    fn SetOperationResult(&self, operation_result: i32) -> Result<(), Error>;
}
impl<T: IArchiveUpdateCallback_Impl> IArchiveUpdateCallback_Ext for T {
    fn GetUpdateItemInfo(&self, index: u32) -> Result<UpdateData, Error> {
        let mut update_data = UpdateData::default();
        unsafe {
            self.GetUpdateItemInfo_Raw(
                index,
                &mut update_data.new_data,
                &mut update_data.new_props,
                &mut update_data.index_in_archive,
            )
                .map(|| update_data)
        }
    }

    fn GetProperty(&self, index: u32, prop_id: PROPID) -> Result<PROPVARIANT, Error> {
        let mut value = PROPVARIANT::default();
        unsafe {
            self.GetProperty_Raw(index, prop_id, &mut value)
                .map(|| value)
        }
    }

    fn GetStream(&self, index: u32) -> Result<ISequentialInStream, Error> {
        let mut in_stream_raw = std::ptr::null_mut();
        unsafe {
            self.GetStream_Raw(index, &mut in_stream_raw)
                .and_then(|| Type::from_abi(in_stream_raw))
        }
    }

    fn SetOperationResult(&self, operation_result: i32) -> Result<(), Error> {
        unsafe {
            self.SetOperationResult_Raw(operation_result)
                .ok()
        }
    }
}

#[interface_7zip(6, 0x82)]
pub unsafe trait IArchiveUpdateCallback2 : IArchiveUpdateCallback {
    fn GetVolumeSize_Raw(&self, index: u32, size: *mut u64) -> HRESULT;
    fn GetVolumeStream_Raw(&self, index: u32, volume_stream: *mut *mut c_void) -> HRESULT;
}

pub trait IArchiveUpdateCallback2_Ext {
    fn GetVolumeSize(&self, index: u32) -> Result<u64, Error>;
    fn GetVolumeStream(&self, index: u32) -> Result<ISequentialOutStream, Error>;
}
impl<T: IArchiveUpdateCallback2_Impl> IArchiveUpdateCallback2_Ext for T {
    fn GetVolumeSize(&self, index: u32) -> Result<u64, Error> {
        let mut size = 0;
        unsafe {
            self.GetVolumeSize_Raw(index, &mut size)
                .map(|| size)
        }
    }

    fn GetVolumeStream(&self, index: u32) -> Result<ISequentialOutStream, Error> {
        let mut volume_stream = std::ptr::null_mut();
        unsafe {
            self.GetVolumeStream_Raw(index, &mut volume_stream)
                .and_then(|| Type::from_abi(volume_stream))
        }
    }
}

#[interface_7zip(6, 0x83)]
pub unsafe trait IArchiveUpdateCallbackFile : IUnknown {
    fn GetStream2_Raw(&self, index: u32, in_stream: *mut *mut c_void, notify_op: u32) -> HRESULT;
    fn ReportOperation_Raw(&self, index_type: u32, index: u32, notify_op: u32) -> HRESULT;
}

pub trait IArchiveUpdateCallbackFile_Ext {
    fn GetStream2(&self, index: u32, notify_op: u32) -> Result<ISequentialInStream, Error>;
    fn ReportOperation(&self, index_type: u32, index: u32, notify_op: u32) -> Result<(), Error>;
}
impl<T: IArchiveUpdateCallbackFile_Impl> IArchiveUpdateCallbackFile_Ext for T {
    fn GetStream2(&self, index: u32, notify_op: u32) -> Result<ISequentialInStream, Error> {
        let mut in_stream = std::ptr::null_mut();
        unsafe {
            self.GetStream2_Raw(index, &mut in_stream, notify_op)
                .and_then(|| Type::from_abi(in_stream))
        }
    }

    fn ReportOperation(&self, index_type: u32, index: u32, notify_op: u32) -> Result<(), Error> {
        unsafe {
            self.ReportOperation_Raw(index_type, index, notify_op)
                .ok()
        }
    }
}

#[interface_7zip(6, 0x84)]
pub unsafe trait IArchiveGetDiskProperty : IUnknown {
    fn GetDiskProperty_Raw(&self, index: u32, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
}

pub trait IArchiveGetDiskProperty_Ext {
    fn GetDiskProperty(&self, index: u32, prop_id: PROPID) -> Result<PROPVARIANT, Error>;
}
impl<T: IArchiveGetDiskProperty_Impl> IArchiveGetDiskProperty_Ext for T {
    fn GetDiskProperty(&self, index: u32, prop_id: PROPID) -> Result<PROPVARIANT, Error> {
        let mut value = PROPVARIANT::default();
        unsafe {
            self.GetDiskProperty_Raw(index, prop_id, &mut value)
                .map(|| value)
        }
    }
}

#[interface_7zip(6, 0xA0)]
pub unsafe trait IOutArchive : IUnknown {
    fn UpdateItems_Raw(&self, out_stream: ISequentialOutStream, num_items: u32, update_callback: IArchiveUpdateCallback) -> HRESULT;
    fn GetFileTimeType_Raw(&self, time_type: *mut u32) -> HRESULT;
}

pub trait IOutArchive_Ext {
    fn UpdateItems(&self, out_stream: ISequentialOutStream, num_items: u32, update_callback: IArchiveUpdateCallback) -> Result<(), Error>;
    fn GetFileTimeType(&self) -> Result<u32, Error>;
}
impl<T: IOutArchive_Impl> IOutArchive_Ext for T {
    fn UpdateItems(&self, out_stream: ISequentialOutStream, num_items: u32, update_callback: IArchiveUpdateCallback) -> Result<(), Error> {
        unsafe {
            self.UpdateItems_Raw(out_stream, num_items, update_callback)
                .ok()
        }
    }

    fn GetFileTimeType(&self) -> Result<u32, Error> {
        let mut time_type = 0;
        unsafe {
            self.GetFileTimeType_Raw(&mut time_type)
                .map(|| time_type)
        }
    }
}

#[interface_7zip(6, 0x30)]
pub unsafe trait ISetProperties : IUnknown {
    fn SetProperties_Raw(&self, names: *const *const wchar_t, values: *const PROPVARIANT, num_props: u32) -> HRESULT;
}

pub trait ISetProperties_Ext {
    fn SetProperties(&self, properties: &[(String, PROPVARIANT)]) -> Result<(), Error>;
}
impl<T: ISetProperties_Impl> ISetProperties_Ext for T {
    fn SetProperties(&self, properties: &[(String, PROPVARIANT)]) -> Result<(), Error> {
        let num_props = properties.len();
        let num_props_u32: u32 = num_props.try_into().unwrap();

        let mut name_values: Vec<Vec<u16>> = Vec::with_capacity(num_props);
        let mut names: Vec<*const wchar_t> = Vec::with_capacity(num_props);
        let mut values: Vec<PROPVARIANT> = Vec::with_capacity(num_props);

        for (n, v) in properties {
            let mut nu16: Vec<wchar_t> = n.encode_utf16()
                .collect();
            nu16.push(0x0000);

            names.push(nu16.as_ptr());
            name_values.push(nu16);

            values.push(v.clone());
        }

        unsafe {
            self.SetProperties_Raw(names.as_ptr(), values.as_ptr(), num_props_u32)
                .ok()
        }
    }
}

#[interface_7zip(6, 0x04)]
pub unsafe trait IArchiveKeepModeForNextOpen : IUnknown {
    fn KeepModeForNextOpen_Raw(&self) -> HRESULT;
}

pub trait IArchiveKeepModeForNextOpen_Ext {
    fn KeepModeForNextOpen(&self) -> Result<(), Error>;
}
impl<T: IArchiveKeepModeForNextOpen_Impl> IArchiveKeepModeForNextOpen_Ext for T {
    fn KeepModeForNextOpen(&self) -> Result<(), Error> {
        unsafe {
            self.KeepModeForNextOpen_Raw()
                .ok()
        }
    }
}

#[interface_7zip(6, 0x05)]
pub unsafe trait IArchiveAllowTail : IUnknown {
    fn AllowTail_Raw(&self, allow_tail: i32) -> HRESULT;
}

pub trait IArchiveAllowTail_Ext {
    fn AllowTail(&self, allow_tail: i32) -> Result<(), Error>;
}
impl<T: IArchiveAllowTail_Impl> IArchiveAllowTail_Ext for T {
    fn AllowTail(&self, allow_tail: i32) -> Result<(), Error> {
        unsafe {
            self.AllowTail_Raw(allow_tail)
                .ok()
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MemoryUseRequestResult {
    pub allowed_size: u64,
    pub answer_flags: u32,
}

#[interface_7zip(6, 0x09)]
pub unsafe trait IArchiveRequestMemoryUseCallback : IUnknown {
    fn RequestMemoryUse_Raw(
        &self,
        flags: u32, index_type: u32, index: u32, path: *const wchar_t,
        required_size: u64, allowed_size: *mut u64, answer_flags: *mut u32,
    ) -> HRESULT;
}

pub trait IArchiveRequestMemoryUseCallback_Ext {
    fn RequestMemoryUse(
        &self,
        flags: u32, index_type: u32, index: u32, path: String,
        required_size: u64,
    ) -> Result<MemoryUseRequestResult, Error>;
}
impl<T: IArchiveRequestMemoryUseCallback_Impl> IArchiveRequestMemoryUseCallback_Ext for T {
    fn RequestMemoryUse(
        &self,
        flags: u32, index_type: u32, index: u32, path: String,
        required_size: u64,
    ) -> Result<MemoryUseRequestResult, Error> {
        let mut murr = MemoryUseRequestResult::default();
        let mut path_vec: Vec<u16> = path.encode_utf16().collect();
        path_vec.push(0x0000);

        unsafe {
            self.RequestMemoryUse_Raw(
                flags, index_type, index, path_vec.as_ptr(),
                required_size, &mut murr.allowed_size, &mut murr.answer_flags,
            )
                .map(|| murr)
        }
    }
}
