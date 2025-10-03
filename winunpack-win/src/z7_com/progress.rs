#![allow(non_camel_case_types, non_snake_case)]


use windows_core::{interface, IUnknown, IUnknown_Vtbl, HRESULT};


#[interface("23170F69-40C1-278A-0000-000000050000")]
pub unsafe trait IProgress : IUnknown {
    fn SetTotal(&self, total: u64) -> HRESULT;
    fn SetCompleted(&self, complete_value: *mut u64) -> HRESULT;
}
