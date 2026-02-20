mod config;
use clap::{Parser, Subcommand};
use config::config::{Config, Created};
use directories::BaseDirs;
use std::io;
use std::path::PathBuf;
use std::process::Command;

static WORKTREES_PATH: &str = "~/Code/worktrees";

pub struct MultiTree {
    config: Config<Created>,
}

impl Default for MultiTree {
    fn default() -> Self {
        let mut config = Config::default().create_config_path();
        let worktrees_dir_path = config.get_worktrees_current_dir_path_string();
        if worktrees_dir_path.is_none() {
            let worktrees_path_buf = expand_tilde(WORKTREES_PATH);
            config.add_worktrees_dir_path(&worktrees_path_buf);
            config.change_worktrees_dir_path(&worktrees_path_buf);
        }
        Self::new(config)
    }
}

impl MultiTree {
    pub fn new(config: Config<Created>) -> Self {
        Self { config }
    }

    pub fn add_worktree(&self, name: String) {
        let path = PathBuf::from(&self.config.get_worktrees_current_dir_path_string().unwrap())
            .join(&name);

        let repo_root = match self.git_root() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("❌ {e}");
                std::process::exit(1);
            }
        };

        let mut cmd = Command::new("git");
        cmd.current_dir(&repo_root);

        if self.branch_exists(&repo_root, &name) {
            cmd.args(["worktree", "add"]).arg(&path).arg(&name);
        } else {
            cmd.args(["worktree", "add", "-b"]).arg(&name).arg(&path);
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
        let path = PathBuf::from(&self.config.get_worktrees_current_dir_path_string().unwrap())
            .join(&name);

        println!("🗑️ Removing worktree:");
        println!("   name: {}", name);
        println!("   path: {}", path.display());

        let status = Command::new("git")
            .args(["worktree", "remove"])
            .arg(&path)
            .status()
            .expect("failed to run `git worktree remove`");

        if !status.success() {
            eprintln!("❌ git worktree remove failed with status: {}", status);
            std::process::exit(status.code().unwrap_or(1));
        }

        println!(
            "🗑️ [stub] Would remove worktree '{}' at {}",
            name,
            path.display()
        );
    }

    pub fn track_worktree(&self, name: String) {
        let path = PathBuf::from(&self.config.get_worktrees_current_dir_path_string().unwrap())
            .join(&name);

        println!("📂 Tracking worktree:");
        println!("   name: {}", name);
        println!("   path: {}", path.display());

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());

        let mut child = Command::new(&shell)
            .current_dir(&path)
            .spawn()
            .expect("failed to spawn shell");

        let status = child.wait().expect("failed to wait on shell");
        if !status.success() {
            eprintln!("⚠ shell exited with status: {}", status);
        }

        println!(
            "📂 [stub] Would enter worktree '{}' at {}",
            name,
            path.display()
        );
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
    Add { name: String },
    Track { name: String },
    Remove { name: String },
}

pub fn run_multitree() {
    let multitree = MultiTree::default();
    let multitree_cli = Cli::parse();

    match multitree_cli.command {
        Commands::Add { name } => multitree.add_worktree(name),
        Commands::Track { name } => multitree.track_worktree(name),
        Commands::Remove { name } => multitree.remove_worktree(name),
    }
}

fn expand_tilde(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        let home = BaseDirs::new()
            .expect("no home dir")
            .home_dir()
            .to_path_buf();
        return home.join(rest);
    }
    PathBuf::from(s)
}
