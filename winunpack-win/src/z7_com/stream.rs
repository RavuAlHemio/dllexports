use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows_core::{Error, HRESULT, IUnknown, IUnknown_Vtbl};
use winunpack_macros::interface_7zip;

use crate::z7_com::{FILETIME, PROPID};


#[interface_7zip(3, 0x01)]
pub unsafe trait ISequentialInStream : IUnknown {
    fn Read_Raw(&self, data: *mut u8, size: u32, processed_size: *mut u32) -> HRESULT;
}

pub trait ISequentialInStream_Ext {
    fn Read(&self, data: &mut [u8]) -> Result<u32, Error>;
}
impl<T: ISequentialInStream_Impl> ISequentialInStream_Ext for T {
    fn Read(&self, data: &mut [u8]) -> Result<u32, Error> {
        let size = data.len().try_into().unwrap();
        let mut processed_size = 0;
        unsafe {
            self.Read_Raw(data.as_mut_ptr(), size, &mut processed_size)
                .map(|| processed_size)
        }
    }
}


#[interface_7zip(3, 0x02)]
pub unsafe trait ISequentialOutStream : IUnknown {
    fn Write_Raw(&self, data: *const u8, size: u32, processed_size: *mut u32) -> HRESULT;
}

pub trait ISequentialOutStream_Ext {
    fn Write(&self, data: &[u8]) -> Result<u32, Error>;
}
impl<T: ISequentialOutStream_Impl> ISequentialOutStream_Ext for T {
    fn Write(&self, data: &[u8]) -> Result<u32, Error> {
        let size = data.len().try_into().unwrap();
        let mut processed_size = 0;
        unsafe {
            self.Write_Raw(data.as_ptr(), size, &mut processed_size)
                .map(|| processed_size)
        }
    }
}

#[interface_7zip(3, 0x03)]
pub unsafe trait IInStream : ISequentialInStream {
    fn Seek_Raw(&self, offset: i64, seek_origin: u32, new_position: *mut u64) -> HRESULT;
}

pub trait IInStream_Ext {
    fn Seek(&self, offset: i64, seek_origin: u32) -> Result<u64, Error>;
}
impl<T: IInStream_Impl> IInStream_Ext for T {
    fn Seek(&self, offset: i64, seek_origin: u32) -> Result<u64, Error> {
        let mut new_position = 0;
        unsafe {
            self.Seek_Raw(offset, seek_origin, &mut new_position)
                .map(|| new_position)
        }
    }
}

#[interface_7zip(3, 0x04)]
pub unsafe trait IOutStream : ISequentialOutStream {
    fn Seek_Raw(&self, offset: i64, seek_origin: u32, new_position: *mut u64) -> HRESULT;
    fn SetSize_Raw(&self, new_size: u64) -> HRESULT;
}

pub trait IOutStream_Ext {
    fn Seek(&self, offset: i64, seek_origin: u32) -> Result<u64, Error>;
    fn SetSize(&self, new_size: u64) -> Result<(), Error>;
}
impl<T: IOutStream_Impl> IOutStream_Ext for T {
    fn Seek(&self, offset: i64, seek_origin: u32) -> Result<u64, Error> {
        let mut new_position = 0;
        unsafe {
            self.Seek_Raw(offset, seek_origin, &mut new_position)
                .map(|| new_position)
        }
    }

    fn SetSize(&self, new_size: u64) -> Result<(), Error> {
        unsafe {
            self.SetSize_Raw(new_size)
                .ok()
        }
    }
}

#[interface_7zip(3, 0x06)]
pub unsafe trait IStreamGetSize : IUnknown {
    fn GetSize_Raw(&self, size: *mut u64) -> HRESULT;
}

pub trait IStreamGetSize_Ext {
    fn GetSize(&self) -> Result<u64, Error>;
}
impl<T: IStreamGetSize_Impl> IStreamGetSize_Ext for T {
    fn GetSize(&self) -> Result<u64, Error> {
        let mut size = 0;
        unsafe {
            self.GetSize_Raw(&mut size)
                .map(|| size)
        }
    }
}

#[interface_7zip(3, 0x07)]
pub unsafe trait IOutStreamFinish : IUnknown {
    fn OutStreamFinish_Raw(&self) -> HRESULT;
}

pub trait IOutStreamFinish_Ext {
    fn OutStreamFinish(&self) -> Result<(), Error>;
}
impl<T: IOutStreamFinish_Impl> IOutStreamFinish_Ext for T {
    fn OutStreamFinish(&self) -> Result<(), Error> {
        unsafe {
            self.OutStreamFinish_Raw()
                .ok()
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StreamProperties {
    pub size: u64,
    pub c_time: FILETIME,
    pub a_time: FILETIME,
    pub m_time: FILETIME,
    pub attrib: u32,
}

#[interface_7zip(3, 0x08)]
pub unsafe trait IStreamGetProps : IUnknown {
    fn GetProps_Raw(&self, size: *mut u64, c_time: *mut FILETIME, a_time: *mut FILETIME, m_time: *mut FILETIME, attrib: *mut u32) -> HRESULT;
}

pub trait IStreamGetProps_Ext {
    fn GetProps(&self) -> Result<StreamProperties, Error>;
}
impl<T: IStreamGetProps_Impl> IStreamGetProps_Ext for T {
    fn GetProps(&self) -> Result<StreamProperties, Error> {
        let mut ret = StreamProperties::default();
        unsafe {
            self.GetProps_Raw(&mut ret.size, &mut ret.c_time, &mut ret.a_time, &mut ret.m_time, &mut ret.attrib)
                .map(|| ret)
        }
    }
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

#[interface_7zip(3, 0x09)]
pub unsafe trait IStreamGetProps2 : IUnknown {
    fn GetProps_Raw(&self, props: *mut CStreamFileProps) -> HRESULT;
}

pub trait IStreamGetProps2_Ext {
    fn GetProps(&self) -> Result<CStreamFileProps, Error>;
}
impl<T: IStreamGetProps2_Impl> IStreamGetProps2_Ext for T {
    fn GetProps(&self) -> Result<CStreamFileProps, Error> {
        let mut ret = CStreamFileProps::default();
        unsafe {
            self.GetProps_Raw(&mut ret)
                .map(|| ret)
        }
    }
}

#[interface_7zip(3, 0x0A)]
pub unsafe trait IStreamGetProp : IUnknown {
    fn GetProperty_Raw(&self, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
    fn ReloadProps_Raw(&self) -> HRESULT;
}

pub trait IStreamGetProp_Ext {
    fn GetProperty(&self, prop_id: PROPID) -> Result<PROPVARIANT, Error>;
    fn ReloadProps(&self) -> Result<(), Error>;
}
impl<T: IStreamGetProp_Impl> IStreamGetProp_Ext for T {
    fn GetProperty(&self, prop_id: PROPID) -> Result<PROPVARIANT, Error> {
        let mut value = PROPVARIANT::default();
        unsafe {
            self.GetProperty_Raw(prop_id, &mut value)
                .map(|| value)
        }
    }

    fn ReloadProps(&self) -> Result<(), Error> {
        unsafe {
            self.ReloadProps_Raw()
                .ok()
        }
    }
}

#[interface_7zip(3, 0x10)]
pub unsafe trait IStreamSetRestriction : IUnknown {
    fn SetRestriction_Raw(&self, begin: u64, end: u64) -> HRESULT;
}

pub trait IStreamSetRestriction_Ext {
    fn SetRestriction(&self, begin: u64, end: u64) -> Result<(), Error>;
}
impl<T: IStreamSetRestriction_Impl> IStreamSetRestriction_Ext for T {
    fn SetRestriction(&self, begin: u64, end: u64) -> Result<(), Error> {
        unsafe {
            self.SetRestriction_Raw(begin, end)
                .ok()
        }
    }
}
