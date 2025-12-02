use clap::{Parser, Subcommand};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Get the root directory of the current git repository.
fn git_root() -> io::Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "not a git repository (or unable to find repo root)",
        ));
    }

    let s = String::from_utf8_lossy(&output.stdout);
    let trimmed = s.trim();
    Ok(PathBuf::from(trimmed))
}

/// Get the default worktree path: sibling "worktrees/<name>" next to the git repo.
///
/// If the repo root is /home/user/Code/monoxide,
/// this returns /home/user/Code/worktrees/<name>.
fn default_worktree_path(name: &str) -> io::Result<PathBuf> {
    let root = git_root()?;
    let parent = root.parent().unwrap_or_else(|| Path::new(".")); // fallback, should almost never happen
    Ok(parent.join("worktrees").join(name))
}

/// For `add`: if a path is provided, use it; otherwise build the default worktrees/<name> path.
fn resolve_add_path(name: &str, path: &Option<String>) -> io::Result<PathBuf> {
    if let Some(p) = path {
        Ok(PathBuf::from(p))
    } else {
        default_worktree_path(name)
    }
}

/// For `track` / `remove`: prefer an existing explicit path; otherwise fall back to
/// default worktrees/<name> if that exists.
fn resolve_existing_path(name: &str, path: &Option<String>) -> io::Result<PathBuf> {
    if let Some(p) = path {
        let pbuf = PathBuf::from(p);
        if pbuf.exists() {
            return Ok(pbuf);
        }
    }

    let default = default_worktree_path(name)?;
    if default.exists() {
        Ok(default)
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "could not find worktree path; tried explicit path and {:?}",
                default
            ),
        ))
    }
}

#[derive(Parser)]
#[command(
    name = "worktree",
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
    /// Add a new worktree.
    ///
    /// If no path is given, it will be placed in a sibling 'worktrees/<name>' folder
    /// next to the current git repo.
    ///
    /// Examples:
    ///   worktree add feature-foo
    ///   worktree add feature-foo /custom/path/feature-foo
    Add {
        /// Name of the branch / worktree
        name: String,
        /// Optional custom path. If omitted, defaults to sibling worktrees/<name>.
        path: Option<String>,
    },

    /// "Track" (open) an existing worktree.
    ///
    /// If path is given and exists, it is used.
    /// Otherwise, it will look for the default sibling 'worktrees/<name>' path.
    ///
    /// A new shell is started in that directory in the same terminal.
    Track { name: String, path: Option<String> },

    /// Remove a worktree.
    ///
    /// Same path resolution as `track`. It calls `git worktree remove <path>`
    /// and also removes the directory if it still exists afterwards.
    Remove { name: String, path: Option<String> },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add { name, path } => {
            let path =
                resolve_add_path(&name, &path).expect("failed to resolve worktree path for add");

            // Ensure the parent directory (worktrees) exists if we're using the default.
            if path.parent().is_some() {
                if let Err(e) = fs::create_dir_all(path.parent().unwrap()) {
                    eprintln!("❌ Failed to create worktrees directory: {e}");
                    std::process::exit(1);
                }
            }

            println!("➕ Adding worktree:");
            println!("   branch: {}", name);
            println!("   path:   {}", path.display());

            let status = Command::new("git")
                .args(["worktree", "add"])
                .arg(&path)
                .arg(&name)
                .status()
                .expect("failed to run `git worktree add`");

            if status.success() {
                println!("✅ git worktree add completed successfully");
            } else {
                eprintln!("❌ git worktree add failed with status: {}", status);
                std::process::exit(status.code().unwrap_or(1));
            }
        }

        Commands::Track { name, path } => {
            let path = resolve_existing_path(&name, &path)
                .expect("failed to resolve existing worktree path for track");

            println!("📂 Tracking worktree:");
            println!("   name: {}", name);
            println!("   path: {}", path.display());

            // Same terminal, but new shell whose cwd is the worktree.
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());

            let mut child = Command::new(&shell)
                .current_dir(&path)
                .spawn()
                .expect("failed to spawn shell");

            let status = child.wait().expect("failed to wait on shell");
            if !status.success() {
                eprintln!("⚠ shell exited with status: {}", status);
            }
        }

        Commands::Remove { name, path } => {
            let path = resolve_existing_path(&name, &path)
                .expect("failed to resolve existing worktree path for remove");

            println!("🗑 Removing worktree:");
            println!("   name: {}", name);
            println!("   path: {}", path.display());

            // First ask git to remove the worktree (this also normally deletes the dir).
            let status = Command::new("git")
                .args(["worktree", "remove"])
                .arg(&path)
                .status()
                .expect("failed to run `git worktree remove`");

            if !status.success() {
                eprintln!("❌ git worktree remove failed with status: {}", status);
                std::process::exit(status.code().unwrap_or(1));
            }

            // If directory still exists for some reason, remove it manually.
            if path.exists() {
                if let Err(e) = fs::remove_dir_all(&path) {
                    eprintln!(
                        "⚠ git removed worktree but directory still exists and could not be deleted: {e}"
                    );
                } else {
                    println!("🧹 Directory removed: {}", path.display());
                }
            }

            println!("✅ Worktree removed");
        }
    }
}

