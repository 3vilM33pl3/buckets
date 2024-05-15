use blake3::Hash;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct CommittedFile {
    pub id: Uuid,
    pub name: String,
    #[serde(serialize_with = "hash_to_hex", deserialize_with = "hex_to_hash")]
    pub hash: Hash,
    pub new: bool,
    pub changed: bool,
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

                // First check if existing files are the same
                for file in self.files.iter() {
                    for other_file in other_commit.files.iter() {
                        if file.name == other_file.name && file.hash != other_file.hash {
                            changes.push(CommittedFile {
                                id: file.id,
                                name: file.name.clone(),
                                hash: file.hash.clone(),
                                new: false,
                                changed: true,
                            });
                        } else if file.name == other_file.name && file.hash == other_file.hash {
                            changes.push(CommittedFile {
                                id: file.id,
                                name: file.name.clone(),
                                hash: other_file.hash.clone(),
                                new: false,
                                changed: false,
                            });
                        }
                    }
                }

                // Add files which haven't changed
                for file in self.files.iter() {
                    let mut found = false;
                    for other_file in other_commit.files.iter() {
                        if file.name == other_file.name {
                            found = true;
                        }
                    }
                    if !found {
                        changes.push(CommittedFile {
                            id: file.id,
                            name: file.name.clone(),
                            hash: file.hash.clone(),
                            new: true,
                            changed: false,
                        });
                    }
                }

                // Check if any changes were found
                if changes.iter().filter(|cf| cf.new).collect::<Vec<_>>().len() > 0 {
                    println!("Changes found.");
                    return Some(changes);
                }

                // Check if any files were removed
                if changes.len() < other_commit.files.len() {
                    println!("Changes found.");
                    return Some(changes);
                }

                None
            }
        }
    }
}

