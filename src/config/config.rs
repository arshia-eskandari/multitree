use super::config_file::{ConfigFile, Saved, Unsaved};
use directories::{BaseDirs, ProjectDirs};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default)]
pub struct Missing;
pub struct Created(PathBuf);

fn config_dir() -> PathBuf {
    let proj = ProjectDirs::from("io", "multitree", "multitree")
        .expect("could not determine config directory");

    proj.config_dir().to_path_buf()
}

fn default_multitree_dir() -> PathBuf {
    let base = BaseDirs::new().expect("Could not determine base directories");
    base.home_dir().to_path_buf()
}

pub struct Config<T> {
    self_dir_path: T,
    file: Option<ConfigFile<Saved>>,
}

impl<T: Default> Default for Config<T> {
    fn default() -> Self {
        Self {
            self_dir_path: T::default(),
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
                    worktrees_dir = ""
                    all_directories = []
                "#,
            )
            .expect("write new config file");
        }

        let contents = std::fs::read_to_string(&config_file_path)
            .expect("failed to read contents from Config.toml");
        let config_file: ConfigFile<Unsaved> =
            toml::from_str(&contents).expect("failed to parse Config.toml");

        let config_file = config_file.save(config_file_path);
        Config::<Created> {
            self_dir_path: Created(self_dir_path),
            file: Some(config_file),
        }
    }
}

impl Config<Created> {
    pub fn get_worktrees_current_dir_path_string(&self) -> Option<String> {
        self.file.as_ref().unwrap().worktrees_dir.clone()
    }

    pub fn add_worktrees_dir_path(&mut self, path: PathBuf) {
        let config_file = self.file.as_mut().unwrap();
        let path_str = path.to_str().unwrap().to_string();

        if config_file.all_directories.contains(&path_str) {
            println!("path already exists");
            return;
        }

        config_file.all_directories.push(path_str.clone());
    }

    pub fn change_worktrees_dir_path(&mut self, path: PathBuf) {
        let config_file = self.file.as_mut().unwrap();
        let path_str = path.to_str().unwrap().to_string();

        if !config_file.all_directories.contains(&path_str) {
            println!("path does not exist");
            return;
        }

        config_file.worktrees_dir = Some(path_str);
    }

    pub fn remove_worktrees_dir_path(&mut self, path: PathBuf) {
        let config_file = self.file.as_mut().unwrap();
        let path_str = path.to_str().unwrap().to_string();
        let index = config_file
            .all_directories
            .iter()
            .position(|x| x == &path_str);

        match index {
            Some(i) => {
                config_file.all_directories.remove(i);
            }
            None => {
                println!("path does not exist")
            }
        }
    }
}
