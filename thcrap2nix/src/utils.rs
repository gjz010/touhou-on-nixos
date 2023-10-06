use std::ffi::CStr;
use std::ptr::null;
use winapi::{shared::minwindef::FARPROC, um::errhandlingapi::GetLastError};

type Error = u32;
pub fn str_from_u8_nul_utf8(utf8_src: &[u8]) -> Result<&str, std::str::Utf8Error> {
    let nul_range_end = utf8_src
        .iter()
        .position(|&c| c == b'\0')
        .unwrap_or(utf8_src.len()); // default to length if no `\0` present
    ::std::str::from_utf8(&utf8_src[0..nul_range_end])
}
pub unsafe fn str_from_pi8_nul_utf8<'a>(p: *const i8) -> Result<&'a str, std::str::Utf8Error> {
    assert_ne!(p, null());
    let cstr = CStr::from_ptr(p);
    str_from_u8_nul_utf8(cstr.to_bytes())
}

pub trait ToResult: Sized {
    fn to_result(&self) -> Result<Self, Error>;
}

impl ToResult for FARPROC {
    fn to_result(&self) -> Result<FARPROC, Error> {
        if *self == std::ptr::null_mut() {
            unsafe { Err(GetLastError()) }
        } else {
            Ok(*self)
        }
    }
}

pub trait IntoNullTerminatedU16 {
    fn to_nullterminated_u16(&self) -> Vec<u16>;
}

impl IntoNullTerminatedU16 for str {
    fn to_nullterminated_u16(&self) -> Vec<u16> {
        self.encode_utf16().chain(Some(0)).collect()
    }
}
