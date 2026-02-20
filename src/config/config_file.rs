use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;

#[derive(Default)]
pub struct Unsaved;
pub struct Saved;

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigFile<T> {
    #[serde(
        default,
        deserialize_with = "empty_string_as_none",
        serialize_with = "none_as_empty_string"
    )]
    pub worktrees_dir: Option<String>,
    #[serde(default)]
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

fn none_as_empty_string<S>(value: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(s) => serializer.serialize_str(s),
        None => serializer.serialize_str(""),
    }
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
