# multitree

A CLI tool for managing Git worktrees with sensible defaults and configurable behavior.

`multitree` wraps common `git worktree` workflows to make adding, listing, removing, and cleaning worktrees a bit more convenient — without abstracting away Git itself.

---

## Features

- Add worktrees (creates branch automatically if needed)
- List existing worktrees
- Open a shell inside a worktree
- Remove worktrees (optional confirmation)
- Clean orphaned worktrees
- Configurable base branch and path resolution
- Optional merge checks and remote pruning before cleanup

---

## Usage

### Add a worktree

```bash
multitree add feature-x
```

If the branch does not exist, it will be created from the configured base branch.

Optional custom path:

```bash
multitree add feature-x --path ../worktrees
```

---

### List worktrees

```bash
multitree list
```

---

### Track (enter) a worktree

```bash
multitree track feature-x
```

This spawns a shell inside the selected worktree.

---

### Remove a worktree

```bash
multitree remove feature-x
```

If confirmation is enabled in config, you will be prompted before removal.

---

### Clean orphaned worktrees

```bash
multitree clean
```

Force cleanup without confirmation:

```bash
multitree clean --force
```

Cleanup respects configuration settings such as:
- Auto-fetch and prune before checking
- Skipping unmerged branches
- Optional deletion of local branches

---

## Configuration

Config file location:

- Linux: `~/.config/multitree/Config.toml`
- macOS: `~/Library/Application Support/multitree/Config.toml`
- Windows: `%APPDATA%\multitree\Config.toml`

Example configuration:

```toml
default_base_branch = "main"

[path]
resolution = "repo_parent" # repo_parent | repo_root | custom
custom_base = ""

[clean]
auto_fetch = true
require_merged = true
delete_local_branch = false

[ui]
preferred_shell = ""
confirm_before_remove = true
```

---

## Path Resolution Modes

| Mode         | Description |
|--------------|-------------|
| repo_parent  | Create worktrees next to the repository |
| repo_root    | Create worktrees inside the repository root |
| custom       | Use a custom base directory |

---

## Testing

`multitree` uses integration tests with real temporary Git repositories to verify behavior for:

- Worktree creation
- Listing
- Removal
- Cleanup logic
- Confirmation handling

Run:

```bash
cargo test
```

---

## License

MIT
