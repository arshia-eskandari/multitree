mod config;
use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use config::config::{Config, ConfigError, Created};
use config::config_file::PathResolution;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

pub struct MultiTree {
    config: Config<Created>,
}

impl Default for MultiTree {
    fn default() -> Self {
        let config = Config::default()
            .create_config_path()
            .expect("failed to create config path");
        Self::new(config)
    }
}

#[derive(Debug, Clone)]
struct WorktreeEntry {
    path: PathBuf,
    branch: Option<String>,
}

#[derive(Debug, Error)]
pub enum MultiTreeError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("not a git repository")]
    NotGitRepo,
    #[error("git command failed: {0}")]
    GitCommandFailed(String),
    #[error("worktree not found for branch `{0}`")]
    WorktreeNotFound(String),
    #[error("failed to parse git worktree list output")]
    ParseWorktreeList,
    #[error("failed to determine repository parent directory")]
    MissingRepoParent,
    #[error("failed to read user input")]
    ReadUserInput(#[source] std::io::Error),
    #[error("failed to spawn shell `{shell}`")]
    SpawnShell {
        shell: String,
        #[source]
        source: std::io::Error,
    },
}

impl MultiTree {
    pub fn new(config: Config<Created>) -> Self {
        Self { config }
    }

    pub fn add_worktree(&self, name: String, path: Option<String>) -> Result<()> {
        let repo_root = self.git_root()?;
        let repo_parent = repo_root
            .parent()
            .ok_or(MultiTreeError::MissingRepoParent)?;

        let base_dir = match path {
            Some(path_str) => {
                let base = PathBuf::from(path_str);
                if base.is_absolute() {
                    base
                } else {
                    repo_parent.join(base)
                }
            }
            None => match self.config.path_config().resolution {
                PathResolution::RepoParent => repo_parent.to_path_buf(),
                PathResolution::RepoRoot => repo_root.clone(),
                PathResolution::Custom => {
                    let custom = self.config.path_config().custom_base.clone();
                    if custom.is_empty() {
                        repo_parent.to_path_buf()
                    } else {
                        let base = PathBuf::from(custom);
                        if base.is_absolute() {
                            base
                        } else {
                            repo_parent.join(base)
                        }
                    }
                }
            },
        };

        let worktree_path = base_dir.join(&name);

        let mut cmd = Command::new("git");
        cmd.current_dir(&repo_root);

        if self.branch_exists(&repo_root, &name)? {
            cmd.args(["worktree", "add"]).arg(&worktree_path).arg(&name);
        } else {
            cmd.args(["worktree", "add", "-b"])
                .arg(&name)
                .arg(&worktree_path)
                .arg(self.config.default_base_branch());
        }

        let out = cmd
            .output()
            .map_err(|e| MultiTreeError::GitCommandFailed(e.to_string()))?;

        if out.status.success() {
            println!("✅ git worktree add completed successfully");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            Err(MultiTreeError::GitCommandFailed(stderr).into())
        }
    }

    pub fn remove_worktree(&self, name: String) -> Result<()> {
        let repo_root = self.git_root()?;
        let entry = self.find_worktree_by_branch(&repo_root, &name)?;

        println!("🗑️ Removing worktree:");
        println!("   name: {}", name);
        println!("   path: {}", entry.path.display());

        if self.config.ui_config().confirm_before_remove && !self.confirm("Proceed? (y/N): ")? {
            return Ok(());
        }

        let status = Command::new("git")
            .current_dir(&repo_root)
            .args(["worktree", "remove"])
            .arg(&entry.path)
            .status()
            .map_err(|e| MultiTreeError::GitCommandFailed(e.to_string()))?;

        if !status.success() {
            bail!(MultiTreeError::GitCommandFailed(format!(
                "git worktree remove failed with status: {}",
                status
            )));
        }

        Ok(())
    }

    pub fn track_worktree(&self, name: String) -> Result<()> {
        let repo_root = self.git_root()?;
        let entry = self.find_worktree_by_branch(&repo_root, &name)?;

        println!("📂 Tracking worktree:");
        println!("   name: {}", name);
        println!("   path: {}", entry.path.display());

        let preferred = self.config.ui_config().preferred_shell.trim().to_string();
        let shell = if !preferred.is_empty() {
            preferred
        } else {
            std::env::var("SHELL").unwrap_or_else(|_| {
                if cfg!(windows) {
                    "cmd".to_string()
                } else {
                    "/bin/bash".to_string()
                }
            })
        };

        let mut child = Command::new(&shell)
            .current_dir(&entry.path)
            .spawn()
            .map_err(|source| MultiTreeError::SpawnShell { shell, source })?;

        let status = child
            .wait()
            .map_err(|e| MultiTreeError::GitCommandFailed(e.to_string()))?;

        if !status.success() {
            eprintln!("⚠ shell exited with status: {}", status);
        }

        Ok(())
    }

    pub fn list_worktrees(&self) -> Result<()> {
        let repo_root = self.git_root()?;
        let entries = self.list_worktrees_porcelain(&repo_root)?;

        println!("📂 Worktrees:");
        for e in entries {
            let branch = e
                .branch
                .as_deref()
                .and_then(|b| b.strip_prefix("refs/heads/"))
                .unwrap_or("(detached)");
            println!("• {}", branch);
            println!("  Path: {}", e.path.display());
        }

        Ok(())
    }

    pub fn clean_worktrees(&self, force: bool) -> Result<()> {
        let repo_root = self.git_root()?;

        if self.config.clean_config().auto_fetch {
            let status = Command::new("git")
                .current_dir(&repo_root)
                .args(["fetch", "--prune"])
                .status()
                .map_err(|e| MultiTreeError::GitCommandFailed(e.to_string()))?;

            if !status.success() {
                bail!(MultiTreeError::GitCommandFailed(format!(
                    "git fetch --prune failed with status: {}",
                    status
                )));
            }
        }

        let base_branch = self.config.default_base_branch().to_string();
        let entries = self.list_worktrees_porcelain(&repo_root)?;

        let mut candidates: Vec<(String, PathBuf)> = Vec::new();

        for e in entries {
            let branch_ref = match &e.branch {
                Some(b) => b,
                None => continue,
            };

            if !branch_ref.starts_with("refs/heads/") {
                continue;
            }

            let branch = branch_ref.strip_prefix("refs/heads/").unwrap().to_string();

            if branch == base_branch {
                continue;
            }

            if self.remote_branch_exists(&repo_root, &branch)? {
                continue;
            }

            if self.config.clean_config().require_merged
                && !self.is_merged_into(&repo_root, &branch, &base_branch)?
            {
                continue;
            }

            candidates.push((branch, e.path));
        }

        if candidates.is_empty() {
            println!("✅ No worktrees to clean");
            return Ok(());
        }

        println!("🧹 Candidates for removal:");
        for (b, p) in &candidates {
            println!("• {}", b);
            println!("  Path: {}", p.display());
        }

        if !force && !self.confirm("Proceed? (y/N): ")? {
            return Ok(());
        }

        for (branch, path) in candidates {
            let status = Command::new("git")
                .current_dir(&repo_root)
                .args(["worktree", "remove"])
                .arg(&path)
                .status()
                .map_err(|e| MultiTreeError::GitCommandFailed(e.to_string()))?;

            if !status.success() {
                eprintln!("❌ failed removing worktree at {}", path.display());
                continue;
            }

            if self.config.clean_config().delete_local_branch {
                let args = if force {
                    ["branch", "-D"]
                } else {
                    ["branch", "-d"]
                };
                let status = Command::new("git")
                    .current_dir(&repo_root)
                    .args(args)
                    .arg(&branch)
                    .status()
                    .map_err(|e| MultiTreeError::GitCommandFailed(e.to_string()))?;

                if !status.success() {
                    eprintln!("❌ failed deleting branch {}", branch);
                }
            }
        }

        Ok(())
    }

    fn list_worktrees_porcelain(&self, repo_root: &PathBuf) -> Result<Vec<WorktreeEntry>> {
        let out = Command::new("git")
            .current_dir(repo_root)
            .args(["worktree", "list", "--porcelain"])
            .output()
            .map_err(|e| MultiTreeError::GitCommandFailed(e.to_string()))?;

        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            return Err(MultiTreeError::GitCommandFailed(stderr).into());
        }

        let text = String::from_utf8_lossy(&out.stdout);

        let mut entries: Vec<WorktreeEntry> = Vec::new();
        let mut current_path: Option<PathBuf> = None;
        let mut current_branch: Option<String> = None;

        for line in text.lines() {
            if line.trim().is_empty() {
                if let Some(p) = current_path.take() {
                    entries.push(WorktreeEntry {
                        path: p,
                        branch: current_branch.take(),
                    });
                }
                continue;
            }

            if let Some(rest) = line.strip_prefix("worktree ") {
                if let Some(p) = current_path.take() {
                    entries.push(WorktreeEntry {
                        path: p,
                        branch: current_branch.take(),
                    });
                }
                current_path = Some(PathBuf::from(rest.trim()));
                current_branch = None;
                continue;
            }

            if let Some(rest) = line.strip_prefix("branch ") {
                current_branch = Some(rest.trim().to_string());
                continue;
            }
        }

        if let Some(p) = current_path.take() {
            entries.push(WorktreeEntry {
                path: p,
                branch: current_branch.take(),
            });
        }

        Ok(entries)
    }

    fn find_worktree_by_branch(&self, repo_root: &PathBuf, name: &str) -> Result<WorktreeEntry> {
        let target = format!("refs/heads/{}", name);
        let entries = self.list_worktrees_porcelain(repo_root)?;

        entries
            .into_iter()
            .find(|e| e.branch.as_deref() == Some(target.as_str()))
            .ok_or_else(|| MultiTreeError::WorktreeNotFound(name.to_string()).into())
    }

    fn remote_branch_exists(&self, repo_root: &PathBuf, branch: &str) -> Result<bool> {
        let status = Command::new("git")
            .current_dir(repo_root)
            .args(["show-ref", "--verify", "--quiet"])
            .arg(format!("refs/remotes/origin/{}", branch))
            .status()
            .map_err(|e| MultiTreeError::GitCommandFailed(e.to_string()))?;

        Ok(status.success())
    }

    fn is_merged_into(&self, repo_root: &PathBuf, branch: &str, base: &str) -> Result<bool> {
        let status = Command::new("git")
            .current_dir(repo_root)
            .args(["merge-base", "--is-ancestor"])
            .arg(branch)
            .arg(base)
            .status()
            .map_err(|e| MultiTreeError::GitCommandFailed(e.to_string()))?;

        Ok(status.success())
    }

    fn confirm(&self, prompt: &str) -> Result<bool> {
        use std::io::Write;
        print!("{}", prompt);
        io::stdout()
            .flush()
            .map_err(MultiTreeError::ReadUserInput)?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(MultiTreeError::ReadUserInput)?;

        Ok(matches!(input.trim(), "y" | "Y" | "yes" | "YES" | "Yes"))
    }

    fn git_root(&self) -> Result<PathBuf> {
        let out = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .map_err(|e| MultiTreeError::GitCommandFailed(e.to_string()))?;

        if !out.status.success() {
            return Err(MultiTreeError::NotGitRepo.into());
        }

        let s = String::from_utf8_lossy(&out.stdout);
        Ok(PathBuf::from(s.trim()))
    }

    fn branch_exists(&self, repo_root: &PathBuf, name: &str) -> Result<bool> {
        let status = Command::new("git")
            .current_dir(repo_root)
            .args(["show-ref", "--verify", "--quiet"])
            .arg(format!("refs/heads/{name}"))
            .status()
            .map_err(|e| MultiTreeError::GitCommandFailed(e.to_string()))?;

        Ok(status.success())
    }
}

#[derive(Parser)]
#[command(
    name = "multitree",
    about = "Manage git worktrees easily",
    version,
    author
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Add {
        name: String,
        #[arg(long)]
        path: Option<String>,
    },
    Track {
        name: String,
    },
    Remove {
        name: String,
    },
    List,
    Clean {
        #[arg(long)]
        force: bool,
    },
}

pub fn run_multitree() {
    let multitree = MultiTree::default();
    let multitree_cli = Cli::parse();

    let res = match multitree_cli.command {
        Commands::Add { name, path } => multitree.add_worktree(name, path),
        Commands::Track { name } => multitree.track_worktree(name),
        Commands::Remove { name } => multitree.remove_worktree(name),
        Commands::List => multitree.list_worktrees(),
        Commands::Clean { force } => multitree.clean_worktrees(force),
    };

    if let Err(e) = res {
        eprintln!("❌ {e}");
        std::process::exit(1);
    }
}
