use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default)]
pub struct Unsaved;
pub struct Saved;

fn default_base_branch() -> String {
    "main".to_string()
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum PathResolution {
    RepoParent,
    RepoRoot,
    Custom,
}

impl Default for PathResolution {
    fn default() -> Self {
        PathResolution::RepoParent
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PathConfig {
    #[serde(default)]
    pub resolution: PathResolution,
    #[serde(default)]
    pub custom_base: String,
}

impl Default for PathConfig {
    fn default() -> Self {
        Self {
            resolution: PathResolution::RepoParent,
            custom_base: String::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CleanConfig {
    #[serde(default = "default_true")]
    pub auto_fetch: bool,
    #[serde(default = "default_true")]
    pub require_merged: bool,
    #[serde(default)]
    pub delete_local_branch: bool,
}

fn default_true() -> bool {
    true
}

impl Default for CleanConfig {
    fn default() -> Self {
        Self {
            auto_fetch: true,
            require_merged: true,
            delete_local_branch: false,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UiConfig {
    #[serde(default)]
    pub preferred_shell: String,
    #[serde(default = "default_true")]
    pub confirm_before_remove: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            preferred_shell: String::new(),
            confirm_before_remove: true,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigFile<T> {
    #[serde(default = "default_base_branch")]
    pub default_base_branch: String,
    #[serde(default)]
    pub path: PathConfig,
    #[serde(default)]
    pub clean: CleanConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(skip)]
    _status: T,
}

impl ConfigFile<Unsaved> {
    pub fn save(self, path: &PathBuf) -> ConfigFile<Saved> {
        let toml_str = toml::to_string_pretty(&self).unwrap();
        std::fs::write(path, toml_str).expect("could not write to Config.toml");

        let ConfigFile {
            default_base_branch,
            path,
            clean,
            ui,
            ..
        } = self;

        ConfigFile {
            default_base_branch,
            path,
            clean,
            ui,
            _status: Saved,
        }
    }
}

impl ConfigFile<Saved> {
    pub fn write(self) -> ConfigFile<Unsaved> {
        let ConfigFile {
            default_base_branch,
            path,
            clean,
            ui,
            ..
        } = self;

        ConfigFile {
            default_base_branch,
            path,
            clean,
            ui,
            _status: Unsaved,
        }
    }
}
