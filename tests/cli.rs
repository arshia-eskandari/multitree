use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tempfile::TempDir;

fn run(cmd: &mut StdCommand) -> (String, String) {
    let out = cmd.output().expect("failed to run command");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() {
        panic!(
            "command failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            out.status, stdout, stderr
        );
    }
    (stdout, stderr)
}

fn git(repo: &Path, args: &[&str]) -> (String, String) {
    let mut cmd = StdCommand::new("git");
    cmd.current_dir(repo).args(args);
    run(&mut cmd)
}

fn init_repo() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().expect("failed to create temp dir");
    let repo = tmp.path().join("repo");
    fs::create_dir_all(&repo).expect("failed to create repo dir");

    git(&repo, &["init"]);
    git(&repo, &["config", "user.email", "test@example.com"]);
    git(&repo, &["config", "user.name", "Test User"]);
    git(&repo, &["checkout", "-b", "main"]);

    fs::write(repo.join("README.md"), "hello\n").expect("failed to write file");
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "init"]);

    (tmp, repo)
}

fn worktree_porcelain(repo: &Path) -> String {
    git(repo, &["worktree", "list", "--porcelain"]).0
}

fn has_branch(porcelain: &str, branch: &str) -> bool {
    let needle = format!("branch refs/heads/{}", branch);
    porcelain.lines().any(|l| l.trim() == needle)
}

fn worktree_path_for_branch(porcelain: &str, branch: &str) -> Option<PathBuf> {
    let branch_line = format!("branch refs/heads/{}", branch);

    let mut current_worktree: Option<PathBuf> = None;

    for line in porcelain.lines() {
        let line = line.trim();
        if line.is_empty() {
            current_worktree = None;
            continue;
        }

        if let Some(rest) = line.strip_prefix("worktree ") {
            current_worktree = Some(PathBuf::from(rest.trim()));
            continue;
        }

        if line == branch_line {
            return current_worktree.clone();
        }
    }

    None
}

fn multitree_cmd() -> Command {
    cargo_bin_cmd!("multitree")
}

#[test]
fn add_creates_worktree_and_branch() {
    let (_tmp, repo) = init_repo();

    let mut cmd = multitree_cmd();
    cmd.current_dir(&repo).args(["add", "feature-x"]);
    cmd.assert().success();

    let porcelain = worktree_porcelain(&repo);
    assert!(has_branch(&porcelain, "feature-x"));

    let wt_path = worktree_path_for_branch(&porcelain, "feature-x").expect("worktree path missing");
    assert!(wt_path.exists());
}

#[test]
fn list_prints_worktrees() {
    let (_tmp, repo) = init_repo();

    let mut cmd = multitree_cmd();
    cmd.current_dir(&repo).args(["add", "feature-x"]);
    cmd.assert().success();

    let mut cmd = multitree_cmd();
    cmd.current_dir(&repo).args(["list"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Worktrees"))
        .stdout(predicate::str::contains("feature-x"));
}

#[test]
fn remove_deletes_worktree_directory_with_confirmation() {
    let (_tmp, repo) = init_repo();

    let mut cmd = multitree_cmd();
    cmd.current_dir(&repo).args(["add", "feature-x"]);
    cmd.assert().success();

    let porcelain = worktree_porcelain(&repo);
    let wt_path = worktree_path_for_branch(&porcelain, "feature-x").expect("worktree path missing");
    assert!(wt_path.exists());

    let mut cmd = multitree_cmd();
    cmd.current_dir(&repo).args(["remove", "feature-x"]);
    cmd.write_stdin("y\n").assert().success();

    assert!(!wt_path.exists());

    let porcelain = worktree_porcelain(&repo);
    assert!(
        !has_branch(&porcelain, "feature-x")
            || worktree_path_for_branch(&porcelain, "feature-x").is_none()
    );
}

#[test]
fn clean_removes_local_orphan_worktree_when_remote_branch_missing() {
    let (_tmp, repo) = init_repo();

    let mut cmd = multitree_cmd();
    cmd.current_dir(&repo).args(["add", "orphan-x"]);
    cmd.assert().success();

    let porcelain = worktree_porcelain(&repo);
    let wt_path = worktree_path_for_branch(&porcelain, "orphan-x").expect("worktree path missing");
    assert!(wt_path.exists());

    let mut cmd = multitree_cmd();
    cmd.current_dir(&repo).args(["clean", "--force"]);
    cmd.assert().success();

    assert!(!wt_path.exists());
}

#[test]
fn remove_cancel_does_not_delete() {
    let (_tmp, repo) = init_repo();

    let mut cmd = multitree_cmd();
    cmd.current_dir(&repo).args(["add", "feature-x"]);
    cmd.assert().success();

    let porcelain = worktree_porcelain(&repo);
    let wt_path = worktree_path_for_branch(&porcelain, "feature-x").expect("worktree path missing");
    assert!(wt_path.exists());

    let mut cmd = multitree_cmd();
    cmd.current_dir(&repo).args(["remove", "feature-x"]);
    cmd.write_stdin("n\n").assert().success();

    assert!(wt_path.exists());

    let porcelain = worktree_porcelain(&repo);
    assert!(has_branch(&porcelain, "feature-x"));
}
