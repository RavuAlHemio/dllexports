#![allow(non_camel_case_types, non_snake_case)]


use std::ffi::c_void;

use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows_core::{HRESULT, interface, IUnknown, IUnknown_Vtbl};

use crate::z7_com::{FILETIME, PROPID};


#[interface("23170F69-40C1-278A-0000-000300010000")]
pub unsafe trait ISequentialInStream : IUnknown {
    fn Read(&self, data: *mut c_void, size: u32, processed_size: *mut u32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000300020000")]
pub unsafe trait ISequentialOutStream : IUnknown {
    fn Write(&self, data: *const c_void, size: u32, processed_size: *mut u32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000300030000")]
pub unsafe trait IInStream : ISequentialInStream {
    fn Seek(&self, offset: i64, seek_origin: u32, new_position: *mut u64) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000300040000")]
pub unsafe trait IOutStream : ISequentialOutStream {
    fn Seek(&self, offset: i64, seek_origin: u32, new_position: *mut u64) -> HRESULT;
    fn SetSize(&self, new_size: u64) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000300060000")]
pub unsafe trait IStreamGetSize : IUnknown {
    fn GetSize(&self, size: *mut u64) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000300070000")]
pub unsafe trait IOutStreamFinish : IUnknown {
    fn OutStreamFinish(&self) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000300080000")]
pub unsafe trait IStreamGetProps : IUnknown {
    fn GetProps(&self, size: *mut u64, c_time: *mut FILETIME, a_time: *mut FILETIME, m_time: *mut FILETIME, attrib: *mut u32) -> HRESULT;
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(C)]
pub struct CStreamFileProps {
    pub size: u64,
    pub volume_id: u64,
    pub file_id_low: u64,
    pub file_id_high: u64,
    pub num_links: u32,
    pub attrib: u32,
    pub c_time: FILETIME,
    pub a_time: FILETIME,
    pub m_time: FILETIME,
}

#[interface("23170F69-40C1-278A-0000-000300090000")]
pub unsafe trait IStreamGetProps2 : IUnknown {
    fn GetProps(&self, props: *mut CStreamFileProps) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-0003000A0000")]
pub unsafe trait IStreamGetProp : IUnknown {
    fn GetProperty(&self, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn ReloadProps(&self) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000300100000")]
pub unsafe trait IStreamSetRestriction : IUnknown {
    fn SetRestriction(&self, begin: u64, end: u64) -> HRESULT;
}
