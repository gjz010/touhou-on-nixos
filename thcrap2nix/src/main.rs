mod bindings;
mod thcrap;
mod thcrapdef;
mod utils;
use clap::arg;
use clap::command;
use clap::Parser;

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use std::collections::VecDeque;
use std::env::remove_var;
use std::env::set_var;
use std::env::var;
use std::sync::Mutex;

#[macro_use]
extern crate log;
use crate::bindings::get_status_t_GET_CANCELLED;
use crate::bindings::get_status_t_GET_CLIENT_ERROR;
use crate::bindings::get_status_t_GET_CRC32_ERROR;
use crate::bindings::get_status_t_GET_DOWNLOADING;
use crate::bindings::get_status_t_GET_OK;
use crate::bindings::get_status_t_GET_SERVER_ERROR;
use crate::bindings::get_status_t_GET_SYSTEM_ERROR;
use crate::thcrap::PatchDesc;
use crate::thcrap::THCrapDLL;
use crate::thcrap::THRepo;
use crate::thcrapdef::THCrapConfig;
use crate::thcrapdef::THCrapDef;
use crate::utils::str_from_pi8_nul_utf8;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg()]
    json: String,
}

fn main() {
    let args = Cli::parse();
    pretty_env_logger::init();
    info!("thcrap2nix");
    trace!("{:?}", std::env::current_dir().unwrap());
    info!("json path = {}", args.json);
    let file = std::fs::read_to_string(&args.json).unwrap();
    let def: THCrapDef = serde_json::de::from_str(&file).unwrap();
    trace!("json = {:?}", &def);
    let thcrap = THCrapDLL::new();
    info!("Fetching thcrap repositories.");
    
    set_var("http_proxy", var("patch_http_proxy").unwrap());
    set_var("https_proxy", var("patch_https_proxy").unwrap());
    set_var("no_proxy", var("patch_NO_PROXY").unwrap());
    
    // Before collecting patches, use thcrap mirror only.
    let mut repo_list= vec![];
    for url in [
        /*
        "https://mirrors.thpatch.net/AsyrafFile/",
        "https://mirrors.thpatch.net/Bravi/",
        "https://mirrors.thpatch.net/Clb184/",
        "https://mirrors.thpatch.net/Clover/",
        "https://mirrors.thpatch.net/DTM/",
        "https://mirrors.thpatch.net/Daichungus/",
        "https://mirrors.thpatch.net/Daikarasu/",
        "https://mirrors.thpatch.net/DedeHead/",
        "https://mirrors.thpatch.net/Gamer251/",
        "https://mirrors.thpatch.net/Gensokyo.EXE/",
        "https://mirrors.thpatch.net/Guy/",
        "https://mirrors.thpatch.net/Kogasas_Mods/",
        "https://mirrors.thpatch.net/LmocinemodPatchRepo/",
        "https://mirrors.thpatch.net/MasterGameFTW3561/",
        "https://mirrors.thpatch.net/MoriyaFaith/",
        "https://mirrors.thpatch.net/Nutzer/",
        "https://mirrors.thpatch.net/PKWeegee/",
        "https://mirrors.thpatch.net/PookChang'e/",
        "https://mirrors.thpatch.net/Priw8/",
        "https://mirrors.thpatch.net/Revenant/",
        "https://mirrors.thpatch.net/RogyWantsCoffee/",
        "https://mirrors.thpatch.net/SMB3Memes/",
        "https://mirrors.thpatch.net/SSM/",
        "https://mirrors.thpatch.net/Shoxlu/",
        "https://mirrors.thpatch.net/Splashman/",
        "https://mirrors.thpatch.net/SuperChrim/",
        "https://mirrors.thpatch.net/TESM/",
        "https://mirrors.thpatch.net/TRDario/",
        "https://mirrors.thpatch.net/Uielicious/",
        "https://mirrors.thpatch.net/UnKnwn/",
        "https://mirrors.thpatch.net/Vasteel/",
        "https://mirrors.thpatch.net/Wast/",
        "https://mirrors.thpatch.net/Wasted/",
        "https://mirrors.thpatch.net/Wensomt/",
        "https://mirrors.thpatch.net/WilliamDavi/",
        "https://mirrors.thpatch.net/WindowDump/",
        "https://mirrors.thpatch.net/catysumi/",
        "https://mirrors.thpatch.net/dass7/",
        "https://mirrors.thpatch.net/egor/",
        "https://mirrors.thpatch.net/farawayvision/",
        "https://mirrors.thpatch.net/mintymods/",
        "https://mirrors.thpatch.net/neonickz/",
        "https://mirrors.thpatch.net/nmlgc/",
        "https://mirrors.thpatch.net/pgj1997/",
        "https://mirrors.thpatch.net/redirectto/",
        "https://mirrors.thpatch.net/shirokura/",
        "https://mirrors.thpatch.net/someguy/",
        "https://mirrors.thpatch.net/sqrt/",
        "https://mirrors.thpatch.net/takuneru/",
        "https://mirrors.thpatch.net/tpZHCHTex/",
        "https://mirrors.thpatch.net/tpZHCNex/",
        "https://mirrors.thpatch.net/wobuffet3/",
        "https://mirrors.thpatch.net/yova/",
        "https://mirrors.thpatch.net/yuureiki/",
        */
        "https://mirrors.thpatch.net/nmlgc/",
        "https://mirrors.thpatch.net/WindowDump/",
    ].into_iter() {
        repo_list.extend(thcrap
            .RepoDiscover_wrapper(url)
            .unwrap());
    }
    info!("Repo Len = {}", repo_list.len());
    let mut search_tree: BTreeMap<String, (&THRepo<'_>, BTreeMap<String, PatchDesc<'_>>)> =
        BTreeMap::new();
    for repo in repo_list.iter() {
        let mut repo_search_tree = BTreeMap::new();
        let id = repo.id().to_owned();
        let patches = repo.patches();
        info!("Repo = {}", id);
        //repo.remove_lilywhite_cc();
        info!("Servers = {:?}", repo.servers());
        for p in patches {
            info!("  {} {}", p.1.patch_id(), p.0);
            let pid = p.1.patch_id();
            repo_search_tree.insert(pid.to_owned(), p.1);
        }
        search_tree.insert(id, (repo, repo_search_tree));
    }
    
    info!("Collecting patches.");
    let mut has_error = false;
    let mut installed: BTreeSet<(String, String)> = BTreeSet::new();
    let mut remaining = VecDeque::new();
    for patch in def.patches.iter() {
        if let Some((_repo, tree)) = search_tree.get(&patch.repo_id) {
            if let Some(desc) = tree.get(&patch.patch_id) {
                remaining.push_back(*desc);
            } else {
                error!("Missing patch: {}/{}", &patch.repo_id, &patch.patch_id);
                has_error = true;
            }
        } else {
            error!("Missing repo id: {}", &patch.repo_id);
            has_error = true;
        }
    }
    if has_error {
        error!("Some specified patches are missing.");
        std::process::exit(1);
    }
    let mut has_error = false;
    let mut archives = vec![];
    while let Some(patch_desc) = remaining.pop_front() {
        let key = (
            patch_desc.repo_id().unwrap().to_owned(),
            patch_desc.patch_id().to_owned(),
        );
        if !installed.contains(&key) {
            info!("Marking patch {}/{} as installed.", &key.0, &key.1);
            let (repo, current_repo_tree) = search_tree.get(&key.0).unwrap();
            let (archive, mut patch) = patch_desc.load_patch(repo);
            info!(">>>>>>");
            info!("Repo servers: {:?}", repo.servers());
            info!("Available servers: {:?}", patch.servers());
            info!("<<<<<<");
            for mut dep in patch.dependencies() {
                // First try to resolve relative.
                if !dep.absolute() {
                    trace!(
                        "Resolving relative dependency: {} relative to {}",
                        dep.patch_id(),
                        &key.0
                    );
                    let did = dep.patch_id();
                    if current_repo_tree.contains_key(did) {
                        trace!("Relative dependency resolved by current repo.");
                        dep.patchdesc.repo_id = patch_desc.patchdesc.repo_id;
                    } else {
                        trace!("Relative dependency resolving by all repos.");
                        for repo in search_tree.iter() {
                            if repo.1 .1.contains_key(did) {
                                trace!("Relative dependency resolved by repo: {}", repo.0);
                                unsafe { dep.patchdesc.repo_id = (*repo.1 .0.repo).id };
                                break;
                            }
                        }
                    }
                }
                if dep.absolute() {
                    if let Some((repo, tree)) = search_tree.get(dep.repo_id().unwrap()) {
                        if let Some(desc) = tree.get(dep.patch_id()) {
                            remaining.push_back(*desc);
                        } else {
                            error!(
                                "Missing patch dependency: {}/{}",
                                &dep.repo_id().unwrap(),
                                &dep.patch_id()
                            );
                            has_error = true;
                        }
                    } else {
                        error!("Missing repo id: {}", &dep.repo_id().unwrap());
                        has_error = true;
                    }
                } else {
                    error!("Unresolved relative dependency: {}!", dep.patch_id());
                    has_error = true;
                }
            }
            patch.add_to_stack();
            archives.push(archive);
            installed.insert(key);
        } else {
            debug!("Dependency already resolved: {}/{}", &key.0, &key.1);
        }
    }
    if has_error {
        error!("Failure detected during patch dependency resolution.");
        std::process::exit(2);
    }
    trace!("Downloading game patches.");
    let file_list: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let touched_file_list: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let has_error = std::sync::atomic::AtomicBool::new(false);
    let error_logs = Mutex::new(Vec::new());
    thcrap.stack_update_wrapper(
        |name| {
            let mut xs = file_list.lock().unwrap();
            xs.push(name.to_owned());
            if !name.contains("/"){
                return true;
            }
            for game in def.games.iter(){
                if name.starts_with(&format!("{}/", game)){
                    return true;
                }
            }

            return false;
        },
        |progress| unsafe {
            let prog = &*progress;
            let patch = str_from_pi8_nul_utf8((*prog.patch).id).unwrap();
            let file = str_from_pi8_nul_utf8(prog.fn_).unwrap();
            match prog.status {
                get_status_t_GET_DOWNLOADING => {
                    let url = str_from_pi8_nul_utf8(prog.url).unwrap();
                    trace!(
                        "{} {} {}/{} Downloading from URL {}",
                        patch,
                        prog.file_progress,
                        prog.file_size,
                        file,
                        url
                    );
                }
                get_status_t_GET_OK => {
                    let url = str_from_pi8_nul_utf8(prog.url).unwrap();
                    let mut touched_file_list = touched_file_list.lock().unwrap();
                    touched_file_list.push(url.to_owned());
                    trace!("{} {} Downloaded", patch, file);
                }
                get_status_t_GET_CLIENT_ERROR => {
                    let error = str_from_pi8_nul_utf8(prog.error).unwrap();
                    let msg = format!("{} {} Client error {}", patch, file, error);
                    error!("{}", &msg);
                    {let mut lock = error_logs.lock().unwrap(); lock.push(msg);}
                    has_error.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                get_status_t_GET_SERVER_ERROR => {
                    let error = str_from_pi8_nul_utf8(prog.error).unwrap();
                    let url = str_from_pi8_nul_utf8(prog.url).unwrap();
                    let msg = format!("{} {} Server error {} {}", patch, file, error, url);
                    error!("{}", &msg);
                    {let mut lock = error_logs.lock().unwrap(); lock.push(msg);}
                    has_error.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                get_status_t_GET_SYSTEM_ERROR => {
                    let error = str_from_pi8_nul_utf8(prog.error).unwrap();
                    let url = str_from_pi8_nul_utf8(prog.url).unwrap();
                    let msg = format!("{} {} System error {} {}", patch, file, error, url);
                    error!("{}", &msg);
                    {let mut lock = error_logs.lock().unwrap(); lock.push(msg);}
                    has_error.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                get_status_t_GET_CRC32_ERROR => {
                    let msg = format!("{} {} CRC32 error", patch, file);
                    error!("{}", &msg);
                    {let mut lock = error_logs.lock().unwrap(); lock.push(msg);}
                    has_error.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                get_status_t_GET_CANCELLED => {
                    trace!("{} {} Cancelled", patch, file);
                    //has_error.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                _ => {}
            }
        },
    );
    if has_error.load(std::sync::atomic::Ordering::Relaxed) {
        error!("Failure detected while downloading.");
        for log in error_logs.into_inner().unwrap().into_iter(){
            error!("{}", log);
        }
        std::process::exit(3);
    }
    archives.reverse();
    let time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    std::fs::write(format!("file_list_{}.log", time), file_list.into_inner().unwrap().join("\n")).unwrap();
    std::fs::write(format!("download_list_{}.log", time), touched_file_list.into_inner().unwrap().join("\n")).unwrap();
    let config_json = serde_json::to_string(&THCrapConfig::from_patches(archives)).unwrap();
    std::fs::write("thcrap2nix.js", config_json).unwrap();
    info!("thcrap update finished.")
    //thcrap.
}

#[no_mangle]
pub extern "C" fn _Unwind_Resume(_ex_obj: *mut ()) {
    // _ex_obj is actually *mut uw::_Unwind_Exception, but it is private
}
