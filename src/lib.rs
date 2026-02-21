mod config;
use clap::{Parser, Subcommand};
use config::config::{Config, Created};
use config::config_file::PathResolution;
use std::io;
use std::path::PathBuf;
use std::process::Command;

pub struct MultiTree {
    config: Config<Created>,
}

impl Default for MultiTree {
    fn default() -> Self {
        let config = Config::default().create_config_path();
        Self::new(config)
    }
}

#[derive(Debug, Clone)]
struct WorktreeEntry {
    path: PathBuf,
    branch: Option<String>,
}

impl MultiTree {
    pub fn new(config: Config<Created>) -> Self {
        Self { config }
    }

    pub fn add_worktree(&self, name: String, path: Option<String>) {
        let repo_root = self.git_root().unwrap();
        let repo_parent = repo_root.parent().unwrap();

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

        if self.branch_exists(&repo_root, &name) {
            cmd.args(["worktree", "add"]).arg(&worktree_path).arg(&name);
        } else {
            cmd.args(["worktree", "add", "-b"])
                .arg(&name)
                .arg(&worktree_path)
                .arg(self.config.default_base_branch());
        }

        let out = cmd.output().expect("failed to run `git worktree add`");

        if out.status.success() {
            println!("✅ git worktree add completed successfully");
        } else {
            eprintln!("❌ git worktree add failed (exit {:?})", out.status.code());
            eprintln!("{}", String::from_utf8_lossy(&out.stderr));
            std::process::exit(out.status.code().unwrap_or(1));
        }
    }

    pub fn remove_worktree(&self, name: String) {
        let repo_root = self.git_root().unwrap();
        let entry = self.find_worktree_by_branch(&repo_root, &name);

        println!("🗑️ Removing worktree:");
        println!("   name: {}", name);
        println!("   path: {}", entry.path.display());

        if self.config.ui_config().confirm_before_remove {
            if !self.confirm("Proceed? (y/N): ") {
                return;
            }
        }

        let status = Command::new("git")
            .current_dir(&repo_root)
            .args(["worktree", "remove"])
            .arg(&entry.path)
            .status()
            .expect("failed to run `git worktree remove`");

        if !status.success() {
            eprintln!("❌ git worktree remove failed with status: {}", status);
            std::process::exit(status.code().unwrap_or(1));
        }
    }

    pub fn track_worktree(&self, name: String) {
        let repo_root = self.git_root().unwrap();
        let entry = self.find_worktree_by_branch(&repo_root, &name);

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
            .expect("failed to spawn shell");

        let status = child.wait().expect("failed to wait on shell");
        if !status.success() {
            eprintln!("⚠ shell exited with status: {}", status);
        }
    }

    pub fn list_worktrees(&self) {
        let repo_root = self.git_root().unwrap();
        let entries = self.list_worktrees_porcelain(&repo_root);

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
    }

    pub fn clean_worktrees(&self, force: bool) {
        let repo_root = self.git_root().unwrap();

        if self.config.clean_config().auto_fetch {
            Command::new("git")
                .current_dir(&repo_root)
                .args(["fetch", "--prune"])
                .status()
                .expect("failed to run `git fetch --prune`");
        }

        let base_branch = self.config.default_base_branch().to_string();
        let entries = self.list_worktrees_porcelain(&repo_root);

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

            if self.remote_branch_exists(&repo_root, &branch) {
                continue;
            }

            if self.config.clean_config().require_merged {
                if !self.is_merged_into(&repo_root, &branch, &base_branch) {
                    continue;
                }
            }

            candidates.push((branch, e.path));
        }

        if candidates.is_empty() {
            println!("✅ No worktrees to clean");
            return;
        }

        println!("🧹 Candidates for removal:");
        for (b, p) in &candidates {
            println!("• {}", b);
            println!("  Path: {}", p.display());
        }

        if !force {
            if !self.confirm("Proceed? (y/N): ") {
                return;
            }
        }

        for (branch, path) in candidates {
            let status = Command::new("git")
                .current_dir(&repo_root)
                .args(["worktree", "remove"])
                .arg(&path)
                .status()
                .expect("failed to run `git worktree remove`");

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
                Command::new("git")
                    .current_dir(&repo_root)
                    .args(args)
                    .arg(&branch)
                    .status()
                    .expect("failed to run `git branch -d/-D`");
            }
        }
    }

    fn list_worktrees_porcelain(&self, repo_root: &PathBuf) -> Vec<WorktreeEntry> {
        let out = Command::new("git")
            .current_dir(repo_root)
            .args(["worktree", "list", "--porcelain"])
            .output()
            .expect("failed to run `git worktree list --porcelain`");

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

        entries
    }

    fn find_worktree_by_branch(&self, repo_root: &PathBuf, name: &str) -> WorktreeEntry {
        let target = format!("refs/heads/{}", name);
        let entries = self.list_worktrees_porcelain(repo_root);

        entries
            .into_iter()
            .find(|e| e.branch.as_deref() == Some(target.as_str()))
            .expect("worktree not found for branch")
    }

    fn remote_branch_exists(&self, repo_root: &PathBuf, branch: &str) -> bool {
        Command::new("git")
            .current_dir(repo_root)
            .args(["show-ref", "--verify", "--quiet"])
            .arg(format!("refs/remotes/origin/{}", branch))
            .status()
            .unwrap()
            .success()
    }

    fn is_merged_into(&self, repo_root: &PathBuf, branch: &str, base: &str) -> bool {
        Command::new("git")
            .current_dir(repo_root)
            .args(["merge-base", "--is-ancestor"])
            .arg(branch)
            .arg(base)
            .status()
            .unwrap()
            .success()
    }

    fn confirm(&self, prompt: &str) -> bool {
        use std::io::Write;
        print!("{}", prompt);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        matches!(input.trim(), "y" | "Y" | "yes" | "YES" | "Yes")
    }

    fn git_root(&self) -> io::Result<PathBuf> {
        let out = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()?;

        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(io::Error::other(format!(
                "not a git repo (git rev-parse failed): {err}"
            )));
        }

        let s = String::from_utf8_lossy(&out.stdout);
        Ok(PathBuf::from(s.trim()))
    }

    fn branch_exists(&self, repo_root: &PathBuf, name: &str) -> bool {
        Command::new("git")
            .current_dir(repo_root)
            .args(["show-ref", "--verify", "--quiet"])
            .arg(format!("refs/heads/{name}"))
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
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

    match multitree_cli.command {
        Commands::Add { name, path } => multitree.add_worktree(name, path),
        Commands::Track { name } => multitree.track_worktree(name),
        Commands::Remove { name } => multitree.remove_worktree(name),
        Commands::List => multitree.list_worktrees(),
        Commands::Clean { force } => multitree.clean_worktrees(force),
    }
}
