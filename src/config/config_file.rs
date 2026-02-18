use directories::{BaseDirs, ProjectDirs};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default)]
pub struct Unsaved;
pub struct Saved;

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigFile<T> {
    pub worktrees_dir: Option<String>,
    pub all_directories: Vec<String>,
    #[serde(skip)]
    status: T,
}

impl ConfigFile<Unsaved> {
    pub fn save(self, path: PathBuf) -> ConfigFile<Saved> {
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
            status: Saved,
        }
    }
}

impl ConfigFile<Saved> {
    pub fn write(
        self,
        worktrees_dir: Option<String>,
        all_directories: Vec<String>,
    ) -> ConfigFile<Unsaved> {
        ConfigFile {
            worktrees_dir,
            all_directories,
            status: Unsaved,
        }
    }
}
