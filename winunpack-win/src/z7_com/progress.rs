use windows_core::{HRESULT, IUnknown, IUnknown_Vtbl};
use winunpack_macros::interface_7zip;


#[interface_7zip(0, 0x05)]
pub unsafe trait IProgress : IUnknown {
    fn SetTotal(&self, total: u64) -> HRESULT;
    fn SetCompleted(&self, complete_value: *mut u64) -> HRESULT;
}
