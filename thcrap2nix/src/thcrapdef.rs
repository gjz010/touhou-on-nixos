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
