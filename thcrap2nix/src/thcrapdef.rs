use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug)]
pub struct THCrapDef {
    pub patches: Vec<PatchDef>,
    pub games: Vec<String>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct PatchDef {
    pub repo_id: String,
    pub patch_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct THCrapConfigPatch{
    pub archive: String
}
#[derive(Serialize, Deserialize, Debug)]
pub struct THCrapConfig{
    pub dat_dump: bool,
    pub patched_files_dump: bool,
    pub patches: Vec<THCrapConfigPatch>
}

impl THCrapConfig{
    pub fn from_patches(archives: Vec<String>)->Self{
        THCrapConfig { dat_dump: false, patched_files_dump: false, patches: archives.into_iter().map(|x| THCrapConfigPatch{archive: x}).collect() }
    }
}