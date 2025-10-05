use windows::Win32::Foundation::FILETIME;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows_core::{BSTR, HRESULT, IUnknown, IUnknown_Vtbl, OutRef};
use winunpack_macros::interface_7zip;

use crate::z7_com::{PROPID, VARTYPE, wchar_t};
use crate::z7_com::progress::{IProgress, IProgress_Impl, IProgress_Vtbl};
use crate::z7_com::stream::IInStream;


#[interface_7zip(8, 0x00)]
pub unsafe trait IFolderFolder : IUnknown {
    fn LoadItems(&self) -> HRESULT;
    fn GetNumberOfItems(&self, num_items: *mut u32) -> HRESULT;
    fn GetProperty(&self, item_index: u32, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn BindToFolderByIndex(&self, index: u32, result_folder: OutRef<IFolderFolder>) -> HRESULT;
    fn BindToFolderByName(&self, name: *const wchar_t, result_folder: OutRef<IFolderFolder>) -> HRESULT;
    fn BindToParentFolder(&self, result_folder: OutRef<IFolderFolder>) -> HRESULT;
    fn GetNumberOfProperties(&self, num_properties: *mut u32) -> HRESULT;
    fn GetPropertyInfo(&self, index: u32, name: *mut BSTR, prop_id: *mut PROPID, var_type: *mut VARTYPE) -> HRESULT;
    fn GetFolderProperty(&self, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
}

#[interface_7zip(8, 0x17)]
pub unsafe trait IFolderAltStreams : IUnknown {
    fn BindToAltStreamsByIndex(&self, index: u32, result_folder: OutRef<IFolderFolder>) -> HRESULT;
    fn BindToAltStreamsByName(&self, name: *const wchar_t, result_folder: OutRef<IFolderFolder>) -> HRESULT;
    fn AreAltStreamsSupported(&self, index: u32, is_supported: *mut i32) -> HRESULT;
}

#[interface_7zip(8, 0x04)]
pub unsafe trait IFolderWasChanged : IUnknown {
    fn WasChanged(&self, was_changed: *mut i32) -> HRESULT;
}

#[interface_7zip(8, 0x0B)]
pub unsafe trait IFolderOperationsExtractCallback : IProgress {
    fn AskWrite(
        &self,
        src_path: *const wchar_t,
        src_is_folder: i32,
        src_time: *const FILETIME,
        src_size: *const u64,
        dest_path_request: *const wchar_t,
        dest_path_result: *mut BSTR,
        write_answer: *mut i32,
    ) -> HRESULT;
    fn ShowMessage(&self, message: *const wchar_t) -> HRESULT;
    fn SetCurrentFilePath(&self, file_path: *const wchar_t) -> HRESULT;
    fn SetNumFiles(&self, num_files: u64) -> HRESULT;
}

#[interface_7zip(8, 0x13)]
pub unsafe trait IFolderOperations : IUnknown {
    fn CreateFolder(&self, name: *const wchar_t, progress: IProgress) -> HRESULT;
    fn CreateFile(&self, name: *const wchar_t, progress: IProgress) -> HRESULT;
    fn Rename(&self, index: u32, new_name: *const wchar_t, progress: IProgress) -> HRESULT;
    fn Delete(&self, indices: *const u32, num_items: u32, progress: IProgress) -> HRESULT;
    fn CopyTo(
        &self, move_mode: i32, indices: *const u32, num_items: u32,
        include_alt_streams: i32, replace_alt_stream_chars_mode: i32,
        path: *const wchar_t, callback: IFolderOperationsExtractCallback,
    ) -> HRESULT;
    fn CopyFrom(
        &self, moveMode: i32, from_folder_path: *const wchar_t,
        items_paths: *const *const wchar_t, num_items: u32, progress: IProgress,
    ) -> HRESULT;
    fn SetProperty(&self, index: u32, prop_id: PROPID, value: *const PROPVARIANT, progress: IProgress) -> HRESULT;
    fn CopyFromFile(&self, index: u32, full_file_path: *const wchar_t, progress: IProgress) -> HRESULT;
}

#[interface_7zip(8, 0x07)]
pub unsafe trait IFolderGetSystemIconIndex : IUnknown {
    fn GetSystemIconIndex(&self, index: u32, icon_index: *mut i32) -> HRESULT;
}

#[interface_7zip(8, 0x08)]
pub unsafe trait IFolderGetItemFullSize : IUnknown {
    fn GetItemFullSize(&self, index: u32, value: *mut PROPVARIANT, progress: IProgress) -> HRESULT;
}

#[interface_7zip(8, 0x14)]
pub unsafe trait IFolderCalcItemFullSize : IUnknown {
    fn CalcItemFullSize(&self, index: u32, progress: IProgress) -> HRESULT;
}

#[interface_7zip(8, 0x09)]
pub unsafe trait IFolderClone : IUnknown {
    fn Clone(&self, result_folder: OutRef<IFolderFolder>) -> HRESULT;
}

#[interface_7zip(8, 0x0A)]
pub unsafe trait IFolderSetFlatMode : IUnknown {
    fn SetFlatMode(&self, flat_mode: i32) -> HRESULT;
}

#[interface_7zip(8, 0x0E)]
pub unsafe trait IFolderProperties : IUnknown {
    fn GetNumberOfFolderProperties(&self, num_properties: *mut u32) -> HRESULT;
    fn GetFolderPropertyInfo(&self, index: u32, name: *mut BSTR, prop_id: *mut PROPID, var_type: *mut VARTYPE) -> HRESULT;
}

#[interface_7zip(8, 0x10)]
pub unsafe trait IFolderArcProps : IUnknown {
    fn GetArcNumLevels(&self, num_levels: *mut u32) -> HRESULT;
    fn GetArcProp(&self, level: u32, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn GetArcNumProps(&self, level: u32, num_props: *mut u32) -> HRESULT;
    fn GetArcPropInfo(&self, level: u32, index: u32, name: *mut BSTR, prop_id: *mut PROPID, var_type: *mut VARTYPE) -> HRESULT;
    fn GetArcProp2(&self, level: u32, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn GetArcNumProps2(&self, level: u32, num_props: *mut u32) -> HRESULT;
    fn GetArcPropInfo2(&self, level: u32, index: u32, name: *mut BSTR, prop_id: *mut PROPID, var_type: *mut VARTYPE) -> HRESULT;
}

#[interface_7zip(8, 0x11)]
pub unsafe trait IGetFolderArcProps : IUnknown {
    fn GetFolderArcProps(&self, object: OutRef<IFolderArcProps>) -> HRESULT;
}

#[interface_7zip(8, 0x15)]
pub unsafe trait IFolderCompare : IUnknown {
    fn CompareItems(&self, index1: u32, index2: u32, prop_id: PROPID, prop_is_raw: i32) -> i32;
}

#[interface_7zip(8, 0x16)]
pub unsafe trait IFolderGetItemName : IUnknown {
    fn GetItemName(&self, index: u32, name: *const *mut wchar_t, len: *mut u32) -> HRESULT;
    fn GetItemPrefix(&self, index: u32, name: *const *mut wchar_t, len: *mut u32) -> HRESULT;
    fn GetItemSize(&self, index: u32) -> u64;
}

#[interface_7zip(9, 0x05)]
pub unsafe trait IFolderManager : IUnknown {
    fn OpenFolderFile(&self, in_stream: IInStream, file_path: *const wchar_t, arc_format: *const wchar_t, result_folder: OutRef<IFolderFolder>, progress: IProgress) -> HRESULT;
    fn GetExtensions(&self, extensions: *mut BSTR) -> HRESULT;
    fn GetIconPath(&self, ext: *const wchar_t, icon_path: *mut BSTR, icon_index: *mut i32) -> HRESULT;
}
