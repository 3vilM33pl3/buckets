use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CommittedFile {
    pub name: String,
    pub size: u64,
    pub md5: String,
}

#[derive(Serialize, Deserialize)]
pub struct Commit {
    pub bucket: String,
    pub files: Vec<CommittedFile>,
    pub timestamp: String,
}

impl Commit {
    pub fn compare_commit(&self, other_commit: &Commit) -> Option<Vec<CommittedFile>> {
        match other_commit {
            Commit {
                bucket: _,
                files: _,
                timestamp: _,
            } => {
                let mut changes = Vec::new();

                for file in self.files.iter() {
                    let mut found = false;
                    for other_file in other_commit.files.iter() {
                        if file.name == other_file.name {
                            found = true;
                            if file.md5 != other_file.md5 {
                                changes.push(CommittedFile {
                                    name: file.name.clone(),
                                    size: 0,
                                    md5: file.md5.clone(),
                                });
                            }
                        }
                    }
                    if !found {
                        changes.push(CommittedFile {
                            name: file.name.clone(),
                            size: 0,
                            md5: file.md5.clone(),
                        });
                    }
                }

                for file in other_commit.files.iter() {
                    let mut found = false;
                    for other_file in self.files.iter() {
                        if file.name == other_file.name {
                            found = true;
                        }
                    }
                    if !found {
                        changes.push(CommittedFile {
                            name: file.name.clone(),
                            size: 0,
                            md5: file.md5.clone(),
                        });
                    }
                }

                if changes.len() > 0 {
                    return Some(changes);
                }
                None
            }
        }
    }
}
