use std::ffi::c_void;
use std::ptr::null_mut;

use windows::Win32::Foundation::{ERROR_NO_UNICODE_TRANSLATION, FILETIME};
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows_core::{BSTR, Error, HRESULT, IUnknown, IUnknown_Vtbl, Type};
use winunpack_macros::interface_7zip;

use crate::z7_com::{PROPID, to_wide_nul_terminated_string, VARTYPE, wchar_t};
use crate::z7_com::archive::PropertyInfo;
use crate::z7_com::progress::{IProgress, IProgress_Impl, IProgress_Vtbl};
use crate::z7_com::stream::IInStream;


#[interface_7zip(8, 0x00)]
pub unsafe trait IFolderFolder : IUnknown {
    fn LoadItems_Raw(&self) -> HRESULT;
    fn GetNumberOfItems_Raw(&self, num_items: *mut u32) -> HRESULT;
    fn GetProperty_Raw(&self, item_index: u32, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn BindToFolderByIndex_Raw(&self, index: u32, result_folder: *mut *mut c_void) -> HRESULT;
    fn BindToFolderByName_Raw(&self, name: *const wchar_t, result_folder: *mut *mut c_void) -> HRESULT;
    fn BindToParentFolder_Raw(&self, result_folder: *mut *mut c_void) -> HRESULT;
    fn GetNumberOfProperties_Raw(&self, num_properties: *mut u32) -> HRESULT;
    fn GetPropertyInfo_Raw(&self, index: u32, name: *mut BSTR, prop_id: *mut PROPID, var_type: *mut VARTYPE) -> HRESULT;
    fn GetFolderProperty_Raw(&self, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
}

pub trait IFolderFolder_Ext {
    fn LoadItems(&self) -> Result<(), Error>;
    fn GetNumberOfItems(&self) -> Result<u32, Error>;
    fn GetProperty(&self, item_index: u32, prop_id: PROPID) -> Result<PROPVARIANT, Error>;
    fn BindToFolderByIndex(&self, index: u32) -> Result<IFolderFolder, Error>;
    fn BindToFolderByName(&self, name: &str) -> Result<IFolderFolder, Error>;
    fn BindToParentFolder(&self) -> Result<IFolderFolder, Error>;
    fn GetNumberOfProperties(&self) -> Result<u32, Error>;
    fn GetPropertyInfo(&self, index: u32) -> Result<PropertyInfo, Error>;
    fn GetFolderProperty(&self, prop_id: PROPID) -> Result<PROPVARIANT, Error>;
}
impl<T: IFolderFolder_Impl> IFolderFolder_Ext for T {
    fn LoadItems(&self) -> Result<(), Error> {
        unsafe {
            self.LoadItems_Raw()
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

    fn GetProperty(&self, item_index: u32, prop_id: PROPID) -> Result<PROPVARIANT, Error> {
        let mut value = PROPVARIANT::default();
        unsafe {
            self.GetProperty_Raw(item_index, prop_id, &mut value)
                .map(|| value)
        }
    }

    fn BindToFolderByIndex(&self, index: u32) -> Result<IFolderFolder, Error> {
        let mut folder: *mut c_void = null_mut();
        unsafe {
            self.BindToFolderByIndex_Raw(index, &mut folder)
                .and_then(|| Type::from_abi(folder))
        }
    }

    fn BindToFolderByName(&self, name: &str) -> Result<IFolderFolder, Error> {
        let name_w = to_wide_nul_terminated_string(name);

        let mut folder: *mut c_void = null_mut();
        unsafe {
            self.BindToFolderByName_Raw(name_w.as_ptr(), &mut folder)
                .and_then(|| Type::from_abi(folder))
        }
    }

    fn BindToParentFolder(&self) -> Result<IFolderFolder, Error> {
        let mut folder: *mut c_void = null_mut();
        unsafe {
            self.BindToParentFolder_Raw(&mut folder)
                .and_then(|| Type::from_abi(folder))
        }
    }

    fn GetNumberOfProperties(&self) -> Result<u32, Error> {
        let mut num_properties = 0;
        unsafe {
            self.GetNumberOfProperties_Raw(&mut num_properties)
                .map(|| num_properties)
        }
    }

    fn GetPropertyInfo(&self, index: u32) -> Result<PropertyInfo, Error> {
        let mut property_info = PropertyInfo::default();
        unsafe {
            self.GetPropertyInfo_Raw(
                index,
                &mut property_info.raw.name,
                &mut property_info.raw.prop_id,
                &mut property_info.var_type,
            )
                .map(|| property_info)
        }
    }

    fn GetFolderProperty(&self, prop_id: PROPID) -> Result<PROPVARIANT, Error> {
        let mut value = PROPVARIANT::default();
        unsafe {
            self.GetFolderProperty_Raw(
                prop_id,
                &mut value,
            )
                .map(|| value)
        }
    }
}

#[interface_7zip(8, 0x17)]
pub unsafe trait IFolderAltStreams : IUnknown {
    fn BindToAltStreamsByIndex_Raw(&self, index: u32, result_folder: *mut *mut c_void) -> HRESULT;
    fn BindToAltStreamsByName_Raw(&self, name: *const wchar_t, result_folder: *mut *mut c_void) -> HRESULT;
    fn AreAltStreamsSupported_Raw(&self, index: u32, is_supported: *mut i32) -> HRESULT;
}

pub trait IFolderAltStreams_Ext {
    fn BindToAltStreamsByIndex(&self, index: u32) -> Result<IFolderFolder, Error>;
    fn BindToAltStreamsByName(&self, name: &str) -> Result<IFolderFolder, Error>;
    fn AreAltStreamsSupported(&self, index: u32) -> Result<i32, Error>;
}
impl<T: IFolderAltStreams_Impl> IFolderAltStreams_Ext for T {
    fn BindToAltStreamsByIndex(&self, index: u32) -> Result<IFolderFolder, Error> {
        let mut result_folder = null_mut();
        unsafe {
            self.BindToAltStreamsByIndex_Raw(index, &mut result_folder)
                .and_then(|| Type::from_abi(result_folder))
        }
    }

    fn BindToAltStreamsByName(&self, name: &str) -> Result<IFolderFolder, Error> {
        let name_w = to_wide_nul_terminated_string(name);

        let mut result_folder = null_mut();
        unsafe {
            self.BindToAltStreamsByName_Raw(name_w.as_ptr(), &mut result_folder)
                .and_then(|| Type::from_abi(result_folder))
        }
    }

    fn AreAltStreamsSupported(&self, index: u32) -> Result<i32, Error> {
        let mut is_supported = 0;
        unsafe {
            self.AreAltStreamsSupported_Raw(index, &mut is_supported)
                .map(|| is_supported)
        }
    }
}

#[interface_7zip(8, 0x04)]
pub unsafe trait IFolderWasChanged : IUnknown {
    fn WasChanged_Raw(&self, was_changed: *mut i32) -> HRESULT;
}

pub trait IFolderWasChanged_Ext {
    fn WasChanged(&self) -> Result<i32, Error>;
}
impl<T: IFolderWasChanged_Impl> IFolderWasChanged_Ext for T {
    fn WasChanged(&self) -> Result<i32, Error> {
        let mut was_changed = 0;
        unsafe {
            self.WasChanged_Raw(&mut was_changed)
                .map(|| was_changed)
        }
    }
}

#[interface_7zip(8, 0x0B)]
pub unsafe trait IFolderOperationsExtractCallback : IProgress {
    fn AskWrite_Raw(
        &self,
        src_path: *const wchar_t,
        src_is_folder: i32,
        src_time: *const FILETIME,
        src_size: *const u64,
        dest_path_request: *const wchar_t,
        dest_path_result: *mut BSTR,
        write_answer: *mut i32,
    ) -> HRESULT;
    fn ShowMessage_Raw(&self, message: *const wchar_t) -> HRESULT;
    fn SetCurrentFilePath_Raw(&self, file_path: *const wchar_t) -> HRESULT;
    fn SetNumFiles_Raw(&self, num_files: u64) -> HRESULT;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WriteResponse {
    pub dest_path_result: BSTR,
    pub write_answer: i32,
}

pub trait IFolderOperationsExtractCallback_Ext {
    fn AskWrite(
        &self,
        src_path: &str,
        src_is_folder: i32,
        src_time: FILETIME,
        src_size: u64,
        dest_path_request: &str,
    ) -> Result<WriteResponse, Error>;
    fn ShowMessage(&self, message: &str) -> Result<(), Error>;
    fn SetCurrentFilePath(&self, file_path: &str) -> Result<(), Error>;
    fn SetNumFiles(&self, num_files: u64) -> Result<(), Error>;
}
impl<T: IFolderOperationsExtractCallback_Impl> IFolderOperationsExtractCallback_Ext for T {
    fn AskWrite(
        &self,
        src_path: &str,
        src_is_folder: i32,
        src_time: FILETIME,
        src_size: u64,
        dest_path_request: &str,
    ) -> Result<WriteResponse, Error> {
        let src_path_w = to_wide_nul_terminated_string(src_path);
        let dest_path_request_w = to_wide_nul_terminated_string(dest_path_request);

        let mut dest_path_result = BSTR::default();
        let mut write_answer = 0;

        unsafe {
            self.AskWrite_Raw(
                src_path_w.as_ptr(),
                src_is_folder,
                &src_time,
                &src_size,
                dest_path_request_w.as_ptr(),
                &mut dest_path_result,
                &mut write_answer,
            )
                .map(|| WriteResponse {
                    dest_path_result,
                    write_answer,
                })
        }
    }

    fn ShowMessage(&self, message: &str) -> Result<(), Error> {
        let message_w = to_wide_nul_terminated_string(message);

        unsafe {
            self.ShowMessage_Raw(message_w.as_ptr())
                .ok()
        }
    }

    fn SetCurrentFilePath(&self, file_path: &str) -> Result<(), Error> {
        let file_path_w = to_wide_nul_terminated_string(file_path);

        unsafe {
            self.SetCurrentFilePath_Raw(file_path_w.as_ptr())
                .ok()
        }
    }

    fn SetNumFiles(&self, num_files: u64) -> Result<(), Error> {
        unsafe {
            self.SetNumFiles_Raw(num_files)
                .ok()
        }
    }
}

#[interface_7zip(8, 0x13)]
pub unsafe trait IFolderOperations : IUnknown {
    fn CreateFolder_Raw(&self, name: *const wchar_t, progress: IProgress) -> HRESULT;
    fn CreateFile_Raw(&self, name: *const wchar_t, progress: IProgress) -> HRESULT;
    fn Rename_Raw(&self, index: u32, new_name: *const wchar_t, progress: IProgress) -> HRESULT;
    fn Delete_Raw(&self, indices: *const u32, num_items: u32, progress: IProgress) -> HRESULT;
    fn CopyTo_Raw(
        &self, move_mode: i32, indices: *const u32, num_items: u32,
        include_alt_streams: i32, replace_alt_stream_chars_mode: i32,
        path: *const wchar_t, callback: IFolderOperationsExtractCallback,
    ) -> HRESULT;
    fn CopyFrom_Raw(
        &self, move_mode: i32, from_folder_path: *const wchar_t,
        items_paths: *const *const wchar_t, num_items: u32, progress: IProgress,
    ) -> HRESULT;
    fn SetProperty_Raw(&self, index: u32, prop_id: PROPID, value: *const PROPVARIANT, progress: IProgress) -> HRESULT;
    fn CopyFromFile_Raw(&self, index: u32, full_file_path: *const wchar_t, progress: IProgress) -> HRESULT;
}

pub trait IFolderOperations_Ext {
    fn CreateFolder(&self, name: &str, progress: IProgress) -> Result<(), Error>;
    fn CreateFile(&self, name: &str, progress: IProgress) -> Result<(), Error>;
    fn Rename(&self, index: u32, new_name: &str, progress: IProgress) -> Result<(), Error>;
    fn Delete(&self, indices: &[u32], progress: IProgress) -> Result<(), Error>;
    fn CopyTo(
        &self, move_mode: i32, indices: &[u32],
        include_alt_streams: i32, replace_alt_stream_chars_mode: i32,
        path: &str, callback: IFolderOperationsExtractCallback,
    ) -> Result<(), Error>;
    fn CopyFrom(
        &self, move_mode: i32, from_folder_path: &str,
        items_paths: &[&str], progress: IProgress,
    ) -> Result<(), Error>;
    fn SetProperty(&self, index: u32, prop_id: PROPID, value: &PROPVARIANT, progress: IProgress) -> Result<(), Error>;
    fn CopyFromFile(&self, index: u32, full_file_path: &str, progress: IProgress) -> Result<(), Error>;
}
impl<T: IFolderOperations_Impl> IFolderOperations_Ext for T {
    fn CreateFolder(&self, name: &str, progress: IProgress) -> Result<(), Error> {
        let name_w = to_wide_nul_terminated_string(name);
        unsafe {
            self.CreateFolder_Raw(name_w.as_ptr(), progress)
                .ok()
        }
    }

    fn CreateFile(&self, name: &str, progress: IProgress) -> Result<(), Error> {
        let name_w = to_wide_nul_terminated_string(name);
        unsafe {
            self.CreateFile_Raw(name_w.as_ptr(), progress)
                .ok()
        }
    }

    fn Rename(&self, index: u32, new_name: &str, progress: IProgress) -> Result<(), Error> {
        let new_name_w = to_wide_nul_terminated_string(new_name);
        unsafe {
            self.Rename_Raw(index, new_name_w.as_ptr(), progress)
                .ok()
        }
    }

    fn Delete(&self, indices: &[u32], progress: IProgress) -> Result<(), Error> {
        let num_items: u32 = indices.len().try_into().unwrap();
        unsafe {
            self.Delete_Raw(indices.as_ptr(), num_items, progress)
                .ok()
        }
    }

    fn CopyTo(
        &self, move_mode: i32, indices: &[u32],
        include_alt_streams: i32, replace_alt_stream_chars_mode: i32,
        path: &str, callback: IFolderOperationsExtractCallback,
    ) -> Result<(), Error> {
        let num_items: u32 = indices.len().try_into().unwrap();
        let path_w = to_wide_nul_terminated_string(path);
        unsafe {
            self.CopyTo_Raw(
                move_mode, indices.as_ptr(), num_items,
                include_alt_streams, replace_alt_stream_chars_mode, path_w.as_ptr(), callback,
            )
                .ok()
        }
    }

    fn CopyFrom(
        &self, move_mode: i32, from_folder_path: &str,
        items_paths: &[&str], progress: IProgress,
    ) -> Result<(), Error> {
        let from_folder_path_w = to_wide_nul_terminated_string(from_folder_path);
        let num_items: u32 = items_paths.len().try_into().unwrap();

        let mut items_paths_w = Vec::with_capacity(items_paths.len());
        for item_path in items_paths {
            items_paths_w.push(to_wide_nul_terminated_string(item_path));
        }

        let items_paths_ptrs: Vec<*const u16> = items_paths_w.iter()
            .map(|ip| ip.as_ptr())
            .collect();

        unsafe {
            self.CopyFrom_Raw(
                move_mode, from_folder_path_w.as_ptr(),
                items_paths_ptrs.as_ptr(), num_items, progress,
            )
                .ok()
        }
    }

    fn SetProperty(&self, index: u32, prop_id: PROPID, value: &PROPVARIANT, progress: IProgress) -> Result<(), Error> {
        unsafe {
            self.SetProperty_Raw(index, prop_id, value, progress)
                .ok()
        }
    }

    fn CopyFromFile(&self, index: u32, full_file_path: &str, progress: IProgress) -> Result<(), Error> {
        let full_file_path_w = to_wide_nul_terminated_string(full_file_path);
        unsafe {
            self.CopyFromFile_Raw(index, full_file_path_w.as_ptr(), progress)
                .ok()
        }
    }
}

#[interface_7zip(8, 0x07)]
pub unsafe trait IFolderGetSystemIconIndex : IUnknown {
    fn GetSystemIconIndex_Raw(&self, index: u32, icon_index: *mut i32) -> HRESULT;
}

pub trait IFolderGetSystemIconIndex_Ext {
    fn GetSystemIconIndex(&self, index: u32) -> Result<i32, Error>;
}
impl<T: IFolderGetSystemIconIndex_Impl> IFolderGetSystemIconIndex_Ext for T {
    fn GetSystemIconIndex(&self, index: u32) -> Result<i32, Error> {
        let mut icon_index = 0;
        unsafe {
            self.GetSystemIconIndex_Raw(index, &mut icon_index)
                .map(|| icon_index)
        }
    }
}

#[interface_7zip(8, 0x08)]
pub unsafe trait IFolderGetItemFullSize : IUnknown {
    fn GetItemFullSize_Raw(&self, index: u32, value: *mut PROPVARIANT, progress: IProgress) -> HRESULT;
}

pub trait IFolderGetItemFullSize_Ext {
    fn GetItemFullSize(&self, index: u32, progress: IProgress) -> Result<PROPVARIANT, Error>;
}
impl<T: IFolderGetItemFullSize_Impl> IFolderGetItemFullSize_Ext for T {
    fn GetItemFullSize(&self, index: u32, progress: IProgress) -> Result<PROPVARIANT, Error> {
        let mut value = PROPVARIANT::default();
        unsafe {
            self.GetItemFullSize_Raw(index, &mut value, progress)
                .map(|| value)
        }
    }
}

#[interface_7zip(8, 0x14)]
pub unsafe trait IFolderCalcItemFullSize : IUnknown {
    fn CalcItemFullSize_Raw(&self, index: u32, progress: IProgress) -> HRESULT;
}

pub trait IFolderCalcItemFullSize_Ext {
    fn GetItemFullSize(&self, index: u32, progress: IProgress) -> Result<(), Error>;
}
impl<T: IFolderCalcItemFullSize_Impl> IFolderCalcItemFullSize_Ext for T {
    fn GetItemFullSize(&self, index: u32, progress: IProgress) -> Result<(), Error> {
        unsafe {
            self.CalcItemFullSize_Raw(index, progress)
                .ok()
        }
    }
}

#[interface_7zip(8, 0x09)]
pub unsafe trait IFolderClone : IUnknown {
    fn Clone_Raw(&self, result_folder: *mut *mut c_void) -> HRESULT;
}

pub trait IFolderClone_Ext {
    fn Clone(&self) -> Result<IFolderFolder, Error>;
}
impl<T: IFolderClone_Impl> IFolderClone_Ext for T {
    fn Clone(&self) -> Result<IFolderFolder, Error> {
        let mut result_folder = null_mut();
        unsafe {
            self.Clone_Raw(&mut result_folder)
                .and_then(|| Type::from_abi(result_folder))
        }
    }
}

#[interface_7zip(8, 0x0A)]
pub unsafe trait IFolderSetFlatMode : IUnknown {
    fn SetFlatMode_Raw(&self, flat_mode: i32) -> HRESULT;
}

pub trait IFolderSetFlatMode_Ext {
    fn SetFlatMode(&self, flat_mode: i32) -> Result<(), Error>;
}
impl<T: IFolderSetFlatMode_Impl> IFolderSetFlatMode_Ext for T {
    fn SetFlatMode(&self, flat_mode: i32) -> Result<(), Error> {
        unsafe {
            self.SetFlatMode_Raw(flat_mode)
                .ok()
        }
    }
}

#[interface_7zip(8, 0x0E)]
pub unsafe trait IFolderProperties : IUnknown {
    fn GetNumberOfFolderProperties_Raw(&self, num_properties: *mut u32) -> HRESULT;
    fn GetFolderPropertyInfo_Raw(&self, index: u32, name: *mut BSTR, prop_id: *mut PROPID, var_type: *mut VARTYPE) -> HRESULT;
}

pub trait IFolderProperties_Ext {
    fn GetNumberOfFolderProperties(&self) -> Result<u32, Error>;
    fn GetFolderPropertyInfo(&self, index: u32) -> Result<PropertyInfo, Error>;
}
impl<T: IFolderProperties_Impl> IFolderProperties_Ext for T {
    fn GetNumberOfFolderProperties(&self) -> Result<u32, Error> {
        let mut num_properties = 0;
        unsafe {
            self.GetNumberOfFolderProperties_Raw(&mut num_properties)
                .map(|| num_properties)
        }
    }

    fn GetFolderPropertyInfo(&self, index: u32) -> Result<PropertyInfo, Error> {
        let mut prop_info = PropertyInfo::default();
        unsafe {
            self.GetFolderPropertyInfo_Raw(
                index,
                &mut prop_info.raw.name,
                &mut prop_info.raw.prop_id,
                &mut prop_info.var_type,
            )
                .map(|| prop_info)
        }
    }
}

#[interface_7zip(8, 0x10)]
pub unsafe trait IFolderArcProps : IUnknown {
    fn GetArcNumLevels_Raw(&self, num_levels: *mut u32) -> HRESULT;
    fn GetArcProp_Raw(&self, level: u32, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn GetArcNumProps_Raw(&self, level: u32, num_props: *mut u32) -> HRESULT;
    fn GetArcPropInfo_Raw(&self, level: u32, index: u32, name: *mut BSTR, prop_id: *mut PROPID, var_type: *mut VARTYPE) -> HRESULT;
    fn GetArcProp2_Raw(&self, level: u32, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn GetArcNumProps2_Raw(&self, level: u32, num_props: *mut u32) -> HRESULT;
    fn GetArcPropInfo2_Raw(&self, level: u32, index: u32, name: *mut BSTR, prop_id: *mut PROPID, var_type: *mut VARTYPE) -> HRESULT;
}

pub trait IFolderArcProps_Ext {
    fn GetArcNumLevels(&self) -> Result<u32, Error>;
    fn GetArcProp(&self, level: u32, prop_id: PROPID) -> Result<PROPVARIANT, Error>;
    fn GetArcNumProps(&self, level: u32) -> Result<u32, Error>;
    fn GetArcPropInfo(&self, level: u32, index: u32) -> Result<PropertyInfo, Error>;
    fn GetArcProp2(&self, level: u32, prop_id: PROPID) -> Result<PROPVARIANT, Error>;
    fn GetArcNumProps2(&self, level: u32) -> Result<u32, Error>;
    fn GetArcPropInfo2(&self, level: u32, index: u32) -> Result<PropertyInfo, Error>;
}
impl<T: IFolderArcProps_Impl> IFolderArcProps_Ext for T {
    fn GetArcNumLevels(&self) -> Result<u32, Error> {
        let mut num_levels = 0;
        unsafe {
            self.GetArcNumLevels_Raw(&mut num_levels)
                .map(|| num_levels)
        }
    }

    fn GetArcProp(&self, level: u32, prop_id: PROPID) -> Result<PROPVARIANT, Error> {
        let mut value = PROPVARIANT::default();
        unsafe {
            self.GetArcProp_Raw(level, prop_id, &mut value)
                .map(|| value)
        }
    }

    fn GetArcNumProps(&self, level: u32) -> Result<u32, Error> {
        let mut num_props = 0;
        unsafe {
            self.GetArcNumProps_Raw(level, &mut num_props)
                .map(|| num_props)
        }
    }

    fn GetArcPropInfo(&self, level: u32, index: u32) -> Result<PropertyInfo, Error> {
        let mut info = PropertyInfo::default();
        unsafe {
            self.GetArcPropInfo_Raw(
                level, index,
                &mut info.raw.name,
                &mut info.raw.prop_id,
                &mut info.var_type,
            )
                .map(|| info)
        }
    }

    fn GetArcProp2(&self, level: u32, prop_id: PROPID) -> Result<PROPVARIANT, Error> {
        let mut value = PROPVARIANT::default();
        unsafe {
            self.GetArcProp2_Raw(level, prop_id, &mut value)
                .map(|| value)
        }
    }

    fn GetArcNumProps2(&self, level: u32) -> Result<u32, Error> {
        let mut num_props = 0;
        unsafe {
            self.GetArcNumProps2_Raw(level, &mut num_props)
                .map(|| num_props)
        }
    }

    fn GetArcPropInfo2(&self, level: u32, index: u32) -> Result<PropertyInfo, Error> {
        let mut info = PropertyInfo::default();
        unsafe {
            self.GetArcPropInfo2_Raw(
                level, index,
                &mut info.raw.name,
                &mut info.raw.prop_id,
                &mut info.var_type,
            )
                .map(|| info)
        }
    }
}

#[interface_7zip(8, 0x11)]
pub unsafe trait IGetFolderArcProps : IUnknown {
    fn GetFolderArcProps_Raw(&self, object: *mut *mut c_void) -> HRESULT;
}

pub trait IGetFolderArcProps_Ext {
    fn GetFolderArcProps(&self) -> Result<IFolderArcProps, Error>;
}
impl<T: IGetFolderArcProps_Impl> IGetFolderArcProps_Ext for T {
    fn GetFolderArcProps(&self) -> Result<IFolderArcProps, Error> {
        let mut object = null_mut();
        unsafe {
            self.GetFolderArcProps_Raw(&mut object)
                .and_then(|| Type::from_abi(object))
        }
    }
}

#[interface_7zip(8, 0x15)]
pub unsafe trait IFolderCompare : IUnknown {
    fn CompareItems_Raw(&self, index1: u32, index2: u32, prop_id: PROPID, prop_is_raw: i32) -> i32;
}

pub trait IFolderCompare_Ext {
    fn CompareItems(&self, index1: u32, index2: u32, prop_id: PROPID, prop_is_raw: i32) -> i32;
}
impl<T: IFolderCompare_Impl> IFolderCompare_Ext for T {
    fn CompareItems(&self, index1: u32, index2: u32, prop_id: PROPID, prop_is_raw: i32) -> i32 {
        unsafe {
            self.CompareItems_Raw(index1, index2, prop_id, prop_is_raw)
        }
    }
}

#[interface_7zip(8, 0x16)]
pub unsafe trait IFolderGetItemName : IUnknown {
    fn GetItemName_Raw(&self, index: u32, name: *mut *const wchar_t, len: *mut u32) -> HRESULT;
    fn GetItemPrefix_Raw(&self, index: u32, name: *mut *const wchar_t, len: *mut u32) -> HRESULT;
    fn GetItemSize_Raw(&self, index: u32) -> u64;
}

pub trait IFolderGetItemName_Ext {
    fn GetItemName(&self, index: u32) -> Result<String, Error>;
    fn GetItemPrefix(&self, index: u32) -> Result<String, Error>;
    fn GetItemSize(&self, index: u32) -> u64;
}
impl<T: IFolderGetItemName_Impl> IFolderGetItemName_Ext for T {
    fn GetItemName(&self, index: u32) -> Result<String, Error> {
        let mut data: *const u16 = std::ptr::null();
        let mut len = 0;

        unsafe {
            self.GetItemName_Raw(index, &mut data, &mut len)
                .ok()?;
        }

        let words = unsafe {
            std::slice::from_raw_parts(data, len.try_into().unwrap())
        };
        String::from_utf16(words)
            .map_err(|_| Error::from_hresult(ERROR_NO_UNICODE_TRANSLATION.to_hresult()))
    }

    fn GetItemPrefix(&self, index: u32) -> Result<String, Error> {
        let mut data: *const u16 = std::ptr::null();
        let mut len = 0;

        unsafe {
            self.GetItemPrefix_Raw(index, &mut data, &mut len)
                .ok()?;
        }

        let words = unsafe {
            std::slice::from_raw_parts(data, len.try_into().unwrap())
        };
        String::from_utf16(words)
            .map_err(|_| Error::from_hresult(ERROR_NO_UNICODE_TRANSLATION.to_hresult()))
    }

    fn GetItemSize(&self, index: u32) -> u64 {
        unsafe {
            self.GetItemSize_Raw(index)
        }
    }
}

#[interface_7zip(9, 0x05)]
pub unsafe trait IFolderManager : IUnknown {
    fn OpenFolderFile_Raw(&self, in_stream: IInStream, file_path: *const wchar_t, arc_format: *const wchar_t, result_folder: *mut *mut c_void, progress: IProgress) -> HRESULT;
    fn GetExtensions_Raw(&self, extensions: *mut BSTR) -> HRESULT;
    fn GetIconPath_Raw(&self, ext: *const wchar_t, icon_path: *mut BSTR, icon_index: *mut i32) -> HRESULT;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IconInfo {
    pub path: BSTR,
    pub index: i32,
}

pub trait IFolderManager_Ext {
    fn OpenFolderFile(&self, in_stream: IInStream, file_path: &str, arc_format: &str, progress: IProgress) -> Result<IFolderFolder, Error>;
    fn GetExtensions(&self) -> Result<BSTR, Error>;
    fn GetIconPath(&self, ext: &str) -> Result<IconInfo, Error>;
}
impl<T: IFolderManager_Impl> IFolderManager_Ext for T {
    fn OpenFolderFile(&self, in_stream: IInStream, file_path: &str, arc_format: &str, progress: IProgress) -> Result<IFolderFolder, Error> {
        let file_path_w = to_wide_nul_terminated_string(file_path);
        let arc_format_w = to_wide_nul_terminated_string(arc_format);

        let mut result_folder = null_mut();
        unsafe {
            self.OpenFolderFile_Raw(in_stream, file_path_w.as_ptr(), arc_format_w.as_ptr(), &mut result_folder, progress)
                .and_then(|| Type::from_abi(result_folder))
        }
    }

    fn GetExtensions(&self) -> Result<BSTR, Error> {
        let mut extensions = BSTR::default();
        unsafe {
            self.GetExtensions_Raw(&mut extensions)
                .map(|| extensions)
        }
    }

    fn GetIconPath(&self, ext: &str) -> Result<IconInfo, Error> {
        let ext_w = to_wide_nul_terminated_string(ext);
        let mut icon_info = IconInfo::default();

        unsafe {
            self.GetIconPath_Raw(
                ext_w.as_ptr(),
                &mut icon_info.path,
                &mut icon_info.index,
            )
                .map(|| icon_info)
        }
    }
}
