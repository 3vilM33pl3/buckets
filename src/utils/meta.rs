use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
pub(crate) struct FileMeta {
    pub name: String,
    pub md5: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Meta {
    pub files: Vec<FileMeta>,
    pub timestamp: String,
}
