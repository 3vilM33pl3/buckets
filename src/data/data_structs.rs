use blake3::Hash;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize)]
pub struct CommittedFile {
    pub name: String,
    #[serde(serialize_with = "hash_to_hex", deserialize_with = "hex_to_hash")]
    pub hash: Hash,
}

#[derive(Serialize, Deserialize)]
pub struct Commit {
    pub bucket: String,
    pub files: Vec<CommittedFile>,
    pub timestamp: String,
}

// Custom function to serialize a `blake3::Hash` to a hex string
fn hash_to_hex<S>(hash: &Hash, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&hash.to_hex())
}

// Custom function to deserialize a hex string back to a `blake3::Hash`
fn hex_to_hash<'de, D>(deserializer: D) -> Result<Hash, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Hash::from_hex(&s).map_err(serde::de::Error::custom)
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
                            if file.hash != other_file.hash {
                                changes.push(CommittedFile {
                                    name: file.name.clone(),
                                    hash: file.hash.clone(),
                                });
                            }
                        }
                    }
                    if !found {
                        changes.push(CommittedFile {
                            name: file.name.clone(),
                            hash: file.hash.clone(),
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
                            hash: file.hash.clone(),
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
