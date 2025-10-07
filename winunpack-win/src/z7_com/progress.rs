use windows_core::{Error, HRESULT, IUnknown, IUnknown_Vtbl};
use winunpack_macros::interface_7zip;


#[interface_7zip(0, 0x05)]
pub unsafe trait IProgress : IUnknown {
    fn SetTotal_Raw(&self, total: u64) -> HRESULT;
    fn SetCompleted_Raw(&self, complete_value: *const u64) -> HRESULT;
}

pub trait IProgress_Ext {
    fn SetTotal(&self, total: u64) -> Result<(), Error>;
    fn SetCompleted(&self, complete_value: u64) -> Result<(), Error>;
}
impl<T: IProgress_Impl> IProgress_Ext for T {
    fn SetTotal(&self, total: u64) -> Result<(), Error> {
        unsafe {
            self.SetTotal_Raw(total)
                .ok()
        }
    }

    fn SetCompleted(&self, complete_value: u64) -> Result<(), Error> {
        unsafe {
            self.SetCompleted_Raw(&complete_value)
                .ok()
        }
    }
}
