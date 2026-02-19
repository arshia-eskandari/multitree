use serde::{Deserialize, Deserializer, Serialize};
use std::path::PathBuf;

#[derive(Default)]
pub struct Unsaved;
pub struct Saved;

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigFile<T> {
    #[serde(deserialize_with = "empty_string_as_none")]
    pub worktrees_dir: Option<String>,
    pub all_directories: Vec<String>,
    #[serde(skip)]
    _status: T,
}

fn empty_string_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;

    Ok(match opt {
        Some(s) if s.trim().is_empty() => None,
        other => other,
    })
}

impl ConfigFile<Unsaved> {
    pub fn save(self, path: &PathBuf) -> ConfigFile<Saved> {
        let toml_str = toml::to_string_pretty(&self).unwrap();

        std::fs::write(path, toml_str).expect("could not write to Config.toml");

        let ConfigFile {
            worktrees_dir,
            all_directories,
            ..
        } = self;

        ConfigFile {
            worktrees_dir,
            all_directories,
            _status: Saved,
        }
    }
}

impl ConfigFile<Saved> {
    pub fn write(self) -> ConfigFile<Unsaved> {
        let ConfigFile {
            worktrees_dir,
            all_directories,
            ..
        } = self;
        ConfigFile {
            worktrees_dir,
            all_directories,
            _status: Unsaved,
        }
    }
}
