mod bindings;
mod thcrapdef;
use bindings::patch_desc_t;
use bindings::patch_t;
use bindings::progress_callback_status_t;
use bindings::progress_callback_t;
use bindings::repo_t;
use clap::Parser;
use clap::arg;
use clap::command;

use winapi::{shared::minwindef::{HMODULE, FARPROC}, ctypes::{c_char, c_void}, um::libloaderapi::{LoadLibraryA, LoadLibraryW, GetProcAddress, FreeLibrary}};
use winapi::um::errhandlingapi::GetLastError;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::env::current_dir;
use std::env::set_current_dir;
use std::env::set_var;
use std::ffi::CStr;
use std::ffi::CString;
use std::os::windows::ffi::OsStrExt;
use std::ffi::OsStr;
use std::ptr::null;
use std::ptr::null_mut;
use jansson_sys::*;
use libc::free;

use crate::bindings::get_status_t_GET_CANCELLED;
use crate::bindings::get_status_t_GET_CLIENT_ERROR;
use crate::bindings::get_status_t_GET_CRC32_ERROR;
use crate::bindings::get_status_t_GET_DOWNLOADING;
use crate::bindings::get_status_t_GET_OK;
use crate::bindings::get_status_t_GET_SERVER_ERROR;
use crate::bindings::get_status_t_GET_SYSTEM_ERROR;
use crate::thcrapdef::THCrapDef;
#[macro_use] extern crate log;
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
    progress_callback: PFPROGRESS_CALLBACK_T,
    progress_param: *mut ::std::os::raw::c_void,
);
type PFREPO_DISCOVER_WRAPPER = extern "cdecl" fn (start_url: *const ::std::os::raw::c_char) -> *mut *mut repo_t;

type PFPRINT_HOOK = extern "cdecl" fn(*const c_char);
type PFNPRINT_HOOK = extern "cdecl" fn(*const c_char, len: usize);

type PFLOG_SET_HOOK = extern "cdecl" fn(PFPRINT_HOOK, PFNPRINT_HOOK);

type PFREPO_FREE = extern "cdecl" fn(repo: *mut repo_t);
type PFPATCH_BOOTSTRAP_WRAPPER = extern "cdecl" fn (sel: *const patch_desc_t, repo: *const repo_t) -> patch_t;
type PFPATCH_INIT = extern "cdecl" fn(
    patch_path: *const ::std::os::raw::c_char,
    patch_info: *const json_t,
    level: usize,
) -> patch_t;

type PFPATCH_FREE = extern "cdecl" fn (patch: *mut patch_t);
type PFSTACK_ADD_PATCH = extern "cdecl" fn (patch: *mut patch_t);

type Error = u32;
pub fn str_from_u8_nul_utf8(utf8_src: &[u8]) -> Result<&str, std::str::Utf8Error> {
    let nul_range_end = utf8_src.iter()
        .position(|&c| c == b'\0')
        .unwrap_or(utf8_src.len()); // default to length if no `\0` present
    ::std::str::from_utf8(&utf8_src[0..nul_range_end])
}
pub unsafe fn str_from_pi8_nul_utf8<'a>(p: *const i8)->Result<&'a str, std::str::Utf8Error>{
    assert_ne!(p, null());
    let cstr = CStr::from_ptr(p);
    str_from_u8_nul_utf8(cstr.to_bytes())
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg()]
    json: String
}

#[derive(Copy, Clone)]
struct PatchDesc<'a>{
    dll: &'a THCrapDLL,
    patchdesc: patch_desc_t
}
impl<'a> PatchDesc<'a>{
    pub fn new(dll: &'a THCrapDLL, patchdesc: patch_desc_t)->Self{
        Self{dll, patchdesc}
    }
    pub fn patch_id(&self)->&str{
        let patch = self.patchdesc;
        unsafe{
            str_from_pi8_nul_utf8(patch.patch_id).unwrap()
        }
    }
    pub fn repo_id(&self)->Option<&str>{
        if self.patchdesc.repo_id == null_mut(){
            return None;
        }
        unsafe{
            Some(str_from_pi8_nul_utf8(self.patchdesc.repo_id).unwrap())
        }
    }
    pub fn absolute(&self)->bool{
        self.patchdesc.repo_id!=null_mut()
    }

    pub fn load_patch(&self, repo: &'a THRepo<'a>)->Patch<'a>{
        let mut p1 = (self.dll.pf_patch_bootstrap_wrapper)(&self.patchdesc as *const _, repo.repo as *const _);
        unsafe {
            info!("Stage 2 for {}/{} archive: {}", str_from_pi8_nul_utf8(self.patchdesc.repo_id).unwrap(), str_from_pi8_nul_utf8(self.patchdesc.patch_id).unwrap(), str_from_pi8_nul_utf8(p1.archive).unwrap());
        }
        let p2 = (self.dll.pf_patch_init)(p1.archive, null(), 0);
        (repo.thcrap.pf_patch_free)(&mut p1 as *mut _);
        Patch::new(repo, p2)
    }
}

struct Patch<'a>{
    repo: &'a THRepo<'a>,
    patch: patch_t
}

impl<'a> Patch<'a>{
    pub fn new(repo: &'a THRepo<'a>, patch: patch_t)->Self{
        Self{repo, patch}
    }
    pub fn patch_id(&self)->&str{
        unsafe{
            str_from_pi8_nul_utf8(self.patch.id).unwrap()
        }
    }
    pub fn to_desc(&self)->PatchDesc<'a>{
        let raw_desc = patch_desc_t{
            repo_id: self.repo.raw_ref().id,
            patch_id: self.patch.id
        };
        PatchDesc::new(self.repo.thcrap, raw_desc)
    }
    pub fn add_to_stack(&mut self)->(){
        (self.repo.thcrap.pf_stack_add_patch)(&mut self.patch as *const _ as *mut _);
    }
    pub fn dependencies(&self)->Vec<PatchDesc<'a>>{
        let mut descs = vec![];
        let mut p = self.patch.dependencies;
        if p==null_mut(){
            return descs;
        }
        unsafe{
            while (*p).patch_id!=null_mut(){
                /*trace!("{}",
                    str_from_pi8_nul_utf8((*p).patch_id).unwrap()
                );*/
                //assert_ne!((*p).repo_id, null_mut());
                descs.push(PatchDesc::new(self.repo.thcrap, *p));
                p=p.add(1);
            }
        }
        descs
    }
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
    pub fn patches(&'a self)->Vec<(&'a str, PatchDesc<'a>)>{
        let repo = self.raw_ref();
        let mut patches = vec![];
        let mut p = repo.patches;
        if p==null_mut(){
            return patches;
        }
        unsafe{
            while (*p).patch_id!=null_mut(){
                patches.push((str_from_pi8_nul_utf8::<'a>((*p).title).unwrap(), PatchDesc::new(self.thcrap, patch_desc_t { 
                    repo_id: (*self.repo).id, patch_id: (*p).patch_id 
                })));
                p=p.add(1);
            }
            
        }
        patches
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
    pf_log_set_hook: PFLOG_SET_HOOK,
    pf_patch_bootstrap_wrapper: PFPATCH_BOOTSTRAP_WRAPPER,
    pf_patch_init: PFPATCH_INIT,
    pf_stack_update_wrapper: PFSTACK_UPDATE_WRAPPER,
    pf_patch_free: PFPATCH_FREE,
    pf_stack_add_patch: PFSTACK_ADD_PATCH
}

pub extern "cdecl" fn print_hook(s: *const c_char){
    unsafe{
        let s = CStr::from_ptr(s as *const _);
        trace!(target: "thcrap_log", "{}", str_from_u8_nul_utf8(s.to_bytes()).unwrap().trim())
    }
}
pub extern "cdecl" fn nprint_hook(s: *const c_char, n: usize){
    unsafe{
        let s = std::slice::from_raw_parts(s as *const u8, n);
        trace!(target: "thcrap_log", "{}", str_from_u8_nul_utf8(s).unwrap().trim())
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
                pf_log_set_hook: load_function!("log_set_hook"),
                pf_patch_bootstrap_wrapper: load_function!("patch_bootstrap_wrapper"),
                pf_patch_init: load_function!("patch_init"),
                pf_stack_update_wrapper:load_function!("stack_update_wrapper"),
                pf_patch_free: load_function!("patch_free"),
                pf_stack_add_patch: load_function!("stack_add_patch")
            };
            (val.pf_log_set_hook)(print_hook, nprint_hook);
            let cwd = current_dir().unwrap();
            val.thcrap_update_module().expect("Failed to load thcrap update module.");
            set_current_dir(cwd).unwrap();
            return val;
        }
    }
    pub fn thcrap_update_module(&self)->Option<()>{
        let x = (self.pf_thcrap_update_module)();
        if x == std::ptr::null_mut(){
            return None;
        }
        return Some(());
    }
    pub fn RepoDiscover_wrapper<'a>(&'a self, start_url: &str)->Option<Vec<THRepo<'a>>>{
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
    pub fn stack_update_wrapper<F: Fn(&str)->bool, P: Fn(*const progress_callback_status_t)->()>(&self, f: F, p: P){
        let box_f = Box::<F>::leak(Box::new(f)) as *mut F;
        let box_p = Box::<P>::leak(Box::new(p)) as *mut P;
        extern "cdecl" fn filter_wrapper<F: Fn(&str)->bool>(
            fn_: *const ::std::os::raw::c_char,
            filter_data: *mut ::std::os::raw::c_void)-> ::std::os::raw::c_int{
            unsafe {
                let p_f = &*std::mem::transmute::<_, *const F>(filter_data);
                let s = str_from_pi8_nul_utf8(fn_).unwrap();
                if (p_f)(s){
                    1
                }else{
                    0
                }
            }
        }
        extern "cdecl" fn progress_wrapper<P: Fn(*const progress_callback_status_t)->()>(status: *mut progress_callback_status_t,
            param: *mut ::std::os::raw::c_void)->bool{
            let p_p = unsafe {&*std::mem::transmute::<_, *const P>(param)};
            (p_p)(status);
            return true;
        }
        (self.pf_stack_update_wrapper)(filter_wrapper::<F>, box_f as *mut _, progress_wrapper::<P>, box_p as *mut _);
        unsafe {
            drop(Box::<F>::from_raw(box_f));
            drop(Box::<P>::from_raw(box_p));
        }
    }
}
impl Drop for THCrapDLL{
    fn drop(&mut self){
        unsafe {FreeLibrary(self.dll);}
    }
}
fn main() {
    let args = Cli::parse();
    set_var("RUST_LOG", "trace");
    set_var("http_proxy", "http://192.168.76.1:30086");
    set_var("https_proxy", "http://192.168.76.1:30086");
    pretty_env_logger::init();
    info!("thcrap2nix");
    trace!("{:?}", std::env::current_dir().unwrap());
    info!("json path = {}", args.json);
    let file = std::fs::read_to_string(&args.json).unwrap();
    let def : THCrapDef= serde_json::de::from_str(&file).unwrap();
    trace!("json = {:?}", &def);
    //set_var("CURLOPT_CAINFO", std::env::var("HOST_SSL_CERT_FILE").unwrap());
    let mut thcrap = THCrapDLL::new();
    info!("Fetching thcrap repositories.");
    let repo_list = thcrap.RepoDiscover_wrapper("https://mirrors.thpatch.net/nmlgc/").unwrap();
    info!("Repo Len = {}", repo_list.len());
    let mut search_tree : BTreeMap<String, (&THRepo<'_>, BTreeMap<String, PatchDesc<'_>>)> = BTreeMap::new();
    for repo in repo_list.iter(){
        let mut repo_search_tree = BTreeMap::new();
        let id = repo.id().to_owned();
        let patches = repo.patches();
        info!("Repo = {}", id);
        for p in patches{
            info!("  {} {}", p.1.patch_id(), p.0);
            let pid = p.1.patch_id();
            repo_search_tree.insert(pid.to_owned(), p.1);
        }
        search_tree.insert(id, (repo, repo_search_tree));
    }
    info!("Collecting patches.");
    let mut has_error = false;
    let mut installed: BTreeSet<(String, String)> = BTreeSet::new();
    let mut remaining = vec![];
    for patch in def.patches.iter(){
        if let Some((repo, tree)) = search_tree.get(&patch.repo_id){
            if let Some(desc) = tree.get(&patch.patch_id){
                remaining.push(*desc);
            }else{
                error!("Missing patch: {}/{}", &patch.repo_id, &patch.patch_id);
                has_error = true;
            }
        }else{
            error!("Missing repo id: {}", &patch.repo_id);
            has_error = true;
        }
    }
    if has_error{
        error!("Some specified patches are missing.");
        std::process::exit(1);
    }
    let mut has_error = false;
    while let Some(patch_desc) = remaining.pop(){
        let key = (patch_desc.repo_id().unwrap().to_owned(), patch_desc.patch_id().to_owned());
        if !installed.contains(&key){
            info!("Installing patch: {}/{}", &key.0, &key.1);
            let (repo, current_repo_tree) = search_tree.get(&key.0).unwrap();
            let mut patch = patch_desc.load_patch(repo);
            for mut dep in patch.dependencies(){
                // First try to resolve relative.
                if !dep.absolute(){
                    trace!("Resolving relative dependency: {} relative to {}", dep.patch_id(), &key.0);
                    let did = dep.patch_id();
                    if current_repo_tree.contains_key(did){
                        trace!("Relative dependency resolved by current repo.");
                        dep.patchdesc.repo_id = patch_desc.patchdesc.repo_id;
                    }else{
                        trace!("Relative dependency resolving by all repos.");
                        for repo in search_tree.iter(){
                            if repo.1.1.contains_key(did){
                                trace!("Relative dependency resolved by repo: {}", repo.0);
                                unsafe {dep.patchdesc.repo_id = (*repo.1.0.repo).id};
                                break;
                            }
                        }
                    }
                }
                if dep.absolute(){
                    if let Some((repo, tree)) = search_tree.get(dep.repo_id().unwrap()){
                        if let Some(desc) = tree.get(dep.patch_id()){
                            remaining.push(*desc);
                        }else{
                            error!("Missing patch dependency: {}/{}", &dep.repo_id().unwrap(), &dep.patch_id());
                            has_error = true;
                        }
                    }else{
                        error!("Missing repo id: {}", &dep.repo_id().unwrap());
                        has_error = true;
                    }
                }else{
                    error!("Unresolved relative dependency: {}!", dep.patch_id());
                    has_error = true;
                }
            }
            patch.add_to_stack();
            installed.insert(key);
        }else{
            debug!("Dependency already resolved: {}/{}", &key.0, &key.1);
        }
    }
    if has_error{
        error!("Failure detected during patch dependency resolution.");
        std::process::exit(2);
    }
    trace!("Downloading game patches.");
    let has_error = std::sync::atomic::AtomicBool::new(false);
    thcrap.stack_update_wrapper(|name|{
        trace!("Filter processing {}", name);
        return !name.contains('/');
    }, |progress|{
        unsafe {
            let prog =&*progress;
            let patch = str_from_pi8_nul_utf8((*prog.patch).id).unwrap();
            let file =  str_from_pi8_nul_utf8(prog.fn_).unwrap();
            match prog.status{
                get_status_t_GET_DOWNLOADING=>{
                    let url = str_from_pi8_nul_utf8(prog.url).unwrap();
                    trace!("{} {} {}/{} Downloading from URL {}", patch, prog.file_progress, prog.file_size, file, url);
                }
                get_status_t_GET_OK=>{
                    trace!("{} {} Downloaded", patch, file);
                }
                get_status_t_GET_CLIENT_ERROR=>{
                    let error =  str_from_pi8_nul_utf8(prog.error).unwrap();
                    error!("{} {} Client error {}", patch, file, error);
                    has_error.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                get_status_t_GET_SERVER_ERROR=>{
                    let error =  str_from_pi8_nul_utf8(prog.error).unwrap();
                    error!("{} {} Server error {}", patch, file, error);
                    has_error.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                get_status_t_GET_SYSTEM_ERROR=>{
                    let error =  str_from_pi8_nul_utf8(prog.error).unwrap();
                    error!("{} {} System error {}", patch, file, error);
                    has_error.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                get_status_t_GET_CRC32_ERROR=>{
                    error!("{} {} CRC32 error", patch, file);
                    has_error.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                get_status_t_GET_CANCELLED=>{
                    trace!("{} {} Cancelled", patch, file);
                }
                _=>{

                }
            }
        
        }
    });
    if has_error.load(std::sync::atomic::Ordering::Relaxed){
        error!("Failure detected while downloading.");
        std::process::exit(3);
    }

    info!("thcrap update finished.")
    //thcrap.
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