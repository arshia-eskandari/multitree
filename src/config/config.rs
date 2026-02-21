use super::config_file::{
    CleanConfig, ConfigFile, PathConfig, PathResolution, Saved, UiConfig, Unsaved,
};
use directories::ProjectDirs;
use std::path::PathBuf;

#[derive(Default)]
pub struct Missing;
pub struct Created(PathBuf);

fn config_dir() -> PathBuf {
    let proj = ProjectDirs::from("io", "multitree", "multitree")
        .expect("could not determine config directory");

    proj.config_dir().to_path_buf()
}

pub struct Config<T> {
    self_file_path: T,
    file: Option<ConfigFile<Saved>>,
}

impl<T: Default> Default for Config<T> {
    fn default() -> Self {
        Self {
            self_file_path: T::default(),
            file: None,
        }
    }
}

impl Config<Missing> {
    pub fn create_config_path(self) -> Config<Created> {
        let self_dir_path = config_dir();
        std::fs::create_dir_all(&self_dir_path).expect("failed to create dir");

        let config_file_path = self_dir_path.join("Config.toml");

        if !config_file_path.exists() {
            std::fs::write(
                &config_file_path,
                r#"
                    default_base_branch = "main"

                    [path]
                    resolution = "repo_parent"
                    custom_base = ""

                    [clean]
                    auto_fetch = true
                    require_merged = true
                    delete_local_branch = false

                    [ui]
                    preferred_shell = ""
                    confirm_before_remove = true
                "#,
            )
            .expect("write new config file");
        }

        let contents = std::fs::read_to_string(&config_file_path)
            .expect("failed to read contents from Config.toml");
        let config_file: ConfigFile<Unsaved> =
            toml::from_str(&contents).expect("failed to parse Config.toml");

        let config_file = config_file.save(&config_file_path);
        Config::<Created> {
            self_file_path: Created(config_file_path),
            file: Some(config_file),
        }
    }
}

impl Config<Created> {
    pub fn config_path(&self) -> &PathBuf {
        &self.self_file_path.0
    }

    pub fn default_base_branch(&self) -> &str {
        &self.file.as_ref().unwrap().default_base_branch
    }

    pub fn path_config(&self) -> &PathConfig {
        &self.file.as_ref().unwrap().path
    }

    pub fn clean_config(&self) -> &CleanConfig {
        &self.file.as_ref().unwrap().clean
    }

    pub fn ui_config(&self) -> &UiConfig {
        &self.file.as_ref().unwrap().ui
    }

    pub fn set_default_base_branch(&mut self, branch: String) {
        let mut config_file = self.file.take().unwrap();
        config_file.default_base_branch = branch;
        let config_file = config_file.write().save(&self.self_file_path.0);
        self.file = Some(config_file);
    }

    pub fn set_path_resolution(&mut self, resolution: PathResolution) {
        let mut config_file = self.file.take().unwrap();
        config_file.path.resolution = resolution;
        let config_file = config_file.write().save(&self.self_file_path.0);
        self.file = Some(config_file);
    }

    pub fn set_custom_base(&mut self, custom_base: String) {
        let mut config_file = self.file.take().unwrap();
        config_file.path.custom_base = custom_base;
        let config_file = config_file.write().save(&self.self_file_path.0);
        self.file = Some(config_file);
    }

    pub fn set_clean_auto_fetch(&mut self, auto_fetch: bool) {
        let mut config_file = self.file.take().unwrap();
        config_file.clean.auto_fetch = auto_fetch;
        let config_file = config_file.write().save(&self.self_file_path.0);
        self.file = Some(config_file);
    }

    pub fn set_clean_require_merged(&mut self, require_merged: bool) {
        let mut config_file = self.file.take().unwrap();
        config_file.clean.require_merged = require_merged;
        let config_file = config_file.write().save(&self.self_file_path.0);
        self.file = Some(config_file);
    }

    pub fn set_clean_delete_local_branch(&mut self, delete_local_branch: bool) {
        let mut config_file = self.file.take().unwrap();
        config_file.clean.delete_local_branch = delete_local_branch;
        let config_file = config_file.write().save(&self.self_file_path.0);
        self.file = Some(config_file);
    }

    pub fn set_preferred_shell(&mut self, preferred_shell: String) {
        let mut config_file = self.file.take().unwrap();
        config_file.ui.preferred_shell = preferred_shell;
        let config_file = config_file.write().save(&self.self_file_path.0);
        self.file = Some(config_file);
    }

    pub fn set_confirm_before_remove(&mut self, confirm_before_remove: bool) {
        let mut config_file = self.file.take().unwrap();
        config_file.ui.confirm_before_remove = confirm_before_remove;
        let config_file = config_file.write().save(&self.self_file_path.0);
        self.file = Some(config_file);
    }
}
