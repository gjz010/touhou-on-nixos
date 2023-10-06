mod bindings;
use bindings::progress_callback_status_t;
use bindings::progress_callback_t;
use bindings::repo_t;
use winapi::{shared::minwindef::{HMODULE, FARPROC}, ctypes::{c_char, c_void}, um::libloaderapi::{LoadLibraryA, LoadLibraryW, GetProcAddress, FreeLibrary}};
use winapi::um::errhandlingapi::GetLastError;
use std::ffi::CStr;
use std::ffi::CString;
use std::os::windows::ffi::OsStrExt;
use std::ffi::OsStr;
use std::ptr::null;
use std::ptr::null_mut;
use jansson_sys::*;
use libc::free;
type PFTHCRAP_UPDATEMODULE = extern "cdecl" fn()->HMODULE;
type PFUPDATE_FILTER_GLOBAL_WRAPPER = extern "cdecl" fn(_fn: *const c_char, *mut c_void);
type PFUPDATE_FILTER_GAMES_WRAPPER = extern "cdecl" fn(_fn: *const c_char, *mut c_void);
type PFPROGRESS_CALLBACK_T = extern "cdecl" fn(
    status: *mut progress_callback_status_t,
    param: *mut ::std::os::raw::c_void,
) -> bool;
type PFUPDATE_FILTER_FUNC_T = extern "cdecl" fn(
    fn_: *const ::std::os::raw::c_char,
    filter_data: *mut ::std::os::raw::c_void,
) -> ::std::os::raw::c_int;
type PFSTACK_UPDATE_WRAPPER = extern "cdecl" fn (
    filter_func: PFUPDATE_FILTER_FUNC_T,
    filter_data: *mut ::std::os::raw::c_void,
    progress_callback: progress_callback_t,
    progress_param: *mut ::std::os::raw::c_void,
);
type PFREPO_DISCOVER_WRAPPER = extern "cdecl" fn (start_url: *const ::std::os::raw::c_char) -> *mut *mut repo_t;

type PFPRINT_HOOK = extern "cdecl" fn(*const c_char);
type PFNPRINT_HOOK = extern "cdecl" fn(*const c_char, len: usize);

type PFLOG_SET_HOOK = extern "cdecl" fn(PFPRINT_HOOK, PFNPRINT_HOOK);

type PFREPO_FREE = extern "cdecl" fn(repo: *mut repo_t);
type Error = u32;
pub fn str_from_u8_nul_utf8(utf8_src: &[u8]) -> Result<&str, std::str::Utf8Error> {
    let nul_range_end = utf8_src.iter()
        .position(|&c| c == b'\0')
        .unwrap_or(utf8_src.len()); // default to length if no `\0` present
    ::std::str::from_utf8(&utf8_src[0..nul_range_end])
}

struct THRepo<'a>{
    thcrap: &'a THCrapDLL,
    repo: *mut repo_t
}
impl<'a> THRepo<'a>{
    pub fn new(thcrap: &'a THCrapDLL, repo: *mut repo_t)->Self{
        Self{thcrap, repo}
    }
    pub fn raw_mut(&mut self)->&mut repo_t{
        unsafe {&mut *self.repo}
    }
    pub fn raw_ref(&self)->&repo_t{
        unsafe {&*self.repo}
    }
    pub fn title(&self)->&str{
        let repo = self.raw_ref();
        unsafe{
            let cstr = CStr::from_ptr(repo.title);
            str_from_u8_nul_utf8(cstr.to_bytes()).unwrap()
        }
    }
    pub fn id(&self)->&str{
        let repo = self.raw_ref();
        unsafe{
            let cstr = CStr::from_ptr(repo.id);
            str_from_u8_nul_utf8(cstr.to_bytes()).unwrap()
        }
    }
}
impl<'a> Drop for THRepo<'a>{
    fn drop(&mut self) {
        self.thcrap.RepoFree(self.repo)
    }
}

struct THCrapDLL{
    dll: HMODULE,
    pf_thcrap_update_module: PFTHCRAP_UPDATEMODULE,
    pf_repodiscover_wrapper: PFREPO_DISCOVER_WRAPPER,
    pf_repofree: PFREPO_FREE,
    pf_log_set_hook: PFLOG_SET_HOOK
}

pub extern "cdecl" fn print_hook(s: *const c_char){
    unsafe{
        let s = CStr::from_ptr(s as *const _);
        println!("{}", str_from_u8_nul_utf8(s.to_bytes()).unwrap())
    }
}
pub extern "cdecl" fn nprint_hook(s: *const c_char, n: usize){
    unsafe{
        let s = std::slice::from_raw_parts(s as *const u8, n);
        println!("{}", str_from_u8_nul_utf8(s).unwrap())
    }
}

impl THCrapDLL{
    pub fn new()->Self{
        let cname = OsStr::new("thcrap.dll")
            .encode_wide()
            .chain(Some(0)) // add NULL termination
            .collect::<Vec<_>>();
        unsafe{
            let module = LoadLibraryW(cname.as_ptr());
            if module == std::ptr::null_mut(){
                panic!("thcrap.dll not found");
            }
            macro_rules! load_function {
                ($symbol:expr) => {
                    {
                        let p = GetProcAddress(module, concat!($symbol, "\0").as_ptr() as *const i8).to_result().unwrap();
                        std::mem::transmute::<_, _>(p)
                    }
                };
            }
            let mut val = Self{
                dll: module, 
                pf_thcrap_update_module: load_function!("thcrap_update_module"),
                pf_repodiscover_wrapper: load_function!("RepoDiscover_wrapper"),
                pf_repofree: load_function!("RepoFree"),
                pf_log_set_hook: load_function!("log_set_hook")
            };
            (val.pf_log_set_hook)(print_hook, nprint_hook);
            val.thcrap_update_module().expect("Failed to load thcrap update module.");
            return val;
        }
    }
    pub fn thcrap_update_module(&mut self)->Option<()>{
        let x = (self.pf_thcrap_update_module)();
        if x == std::ptr::null_mut(){
            return None;
        }
        return Some(());
    }
    pub fn RepoDiscover_wrapper<'a>(&'a mut self, start_url: &str)->Option<Vec<THRepo<'a>>>{
        unsafe {
            let cstr = CString::new(start_url).unwrap();
            let mut ptr = (self.pf_repodiscover_wrapper)(cstr.as_ptr());
            if ptr==null_mut(){
                return None;
            }
            let mut ret = vec![];
            let orig_ptr = ptr;
            while *ptr!=null_mut(){
                ret.push(THRepo::new(self, *ptr));
                ptr=ptr.add(1);   
            }
            free(orig_ptr as *mut _);
            return Some(ret);
        }
    }
    pub fn RepoFree(&self, repo: *mut repo_t){
        unsafe {(self.pf_repofree)(repo)};
    }
}
impl Drop for THCrapDLL{
    fn drop(&mut self){
        unsafe {FreeLibrary(self.dll);}
    }
}
fn main() {
    println!("Hello, world!");
    let mut thcrap = THCrapDLL::new();
    let repo_list = thcrap.RepoDiscover_wrapper("https://mirrors.thpatch.net/nmlgc/").unwrap();
    println!("Len = {}", repo_list.len());
    for repo in repo_list.iter(){
        println!("{}", repo.id());
    }
    println!("{:?}", std::env::current_dir().unwrap());
    unsafe{
        let json = json_object();
        json_decref(json);
    }
    
}

#[no_mangle]
pub extern "C" fn _Unwind_Resume(_ex_obj: *mut ()) {
    // _ex_obj is actually *mut uw::_Unwind_Exception, but it is private
}


trait ToResult: Sized {
    fn to_result(&self) -> Result<Self, Error>;
}

impl ToResult for FARPROC {
    fn to_result(&self) -> Result<FARPROC, Error> {
        if *self == std::ptr::null_mut() {
            unsafe {
                Err(GetLastError())
            }
        } else {
            Ok(*self)
        }
    }
}

trait IntoNullTerminatedU16 {
    fn to_nullterminated_u16(&self) -> Vec<u16>;
}

impl IntoNullTerminatedU16 for str {
    fn to_nullterminated_u16(&self) -> Vec<u16> {
        self.encode_utf16().chain(Some(0)).collect()
    }
}