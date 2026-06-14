use std::path::Path;
use chrono::TimeZone;
use git2::{
    build::CheckoutBuilder, AnnotatedCommit, FetchOptions, IndexAddOption, PushOptions,
    RemoteCallbacks, Repository, Signature,
};

// ── Public types (unchanged; commands.rs and api.ts depend on these) ──────────

#[derive(Debug, serde::Serialize)]
pub struct GitResult {
    pub success: bool,
    pub output: String,
}

#[derive(Debug, serde::Serialize)]
pub struct CommitInfo {
    pub hash: String,
    pub message: String,
    pub timestamp: String,
    pub author: String,
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn open_repo(vault_path: &Path) -> Result<Repository, String> {
    Repository::open(vault_path)
        .map_err(|e| format!("Failed to open git repository: {}", e))
}

/// Reads the committer/author identity from the local git config, falling back
/// to a sensible default so vaults with no user.name / user.email still work.
fn get_signature(repo: &Repository) -> Result<Signature<'static>, String> {
    repo.signature()
        .or_else(|_| Signature::now("Cabbage", "cabbage@local"))
        .map_err(|e| e.to_string())
}

/// Returns the name of the currently checked-out branch (e.g. "main", "master").
fn current_branch(repo: &Repository) -> Result<String, String> {
    let head = repo.head().map_err(|_| "No HEAD reference found".to_string())?;
    let name = head
        .shorthand()
        .map_err(|e| e.to_string())?;
    Ok(name.to_string())
}

/// Credential callback used for every network operation.
/// Priority order:
///   1. SSH key from the running ssh-agent.
///   2. SSH key files at conventional paths (~/.ssh/id_ed25519, id_ecdsa, id_rsa).
///   3. Platform default credentials (NTLM / Negotiate on Windows, etc.).
fn credential_callback(
    _url: &str,
    username: Option<&str>,
    allowed: git2::CredentialType,
) -> Result<git2::Cred, git2::Error> {
    let user = username.unwrap_or("git");

    if allowed.contains(git2::CredentialType::SSH_KEY) {
        if let Ok(cred) = git2::Cred::ssh_key_from_agent(user) {
            return Ok(cred);
        }
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_default();
        for name in &["id_ed25519", "id_ecdsa", "id_rsa"] {
            let key = std::path::PathBuf::from(&home).join(".ssh").join(name);
            if key.exists() {
                if let Ok(cred) = git2::Cred::ssh_key(user, None, &key, None) {
                    return Ok(cred);
                }
            }
        }
    }

    if allowed.contains(git2::CredentialType::DEFAULT) {
        return git2::Cred::default();
    }

    Err(git2::Error::from_str(
        "No suitable credentials found. \
         Configure an ssh-agent, place an SSH key at ~/.ssh/ or use HTTPS with a \
         credential helper.",
    ))
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Initialises a git repository at `vault_path` if one does not already exist.
pub fn ensure_git_repo(vault_path: &Path) -> Result<(), String> {
    if !vault_path.join(".git").exists() {
        Repository::init(vault_path)
            .map_err(|e| format!("git init failed: {}", e))?;
    }
    Ok(())
}

/// Stages `file_path` (or all changes when `file_path == "--all"`) and creates
/// a local commit. Silently succeeds when the index already matches HEAD.
pub fn auto_commit(vault_path: &Path, file_path: &str) -> Result<(), String> {
    let repo = open_repo(vault_path)?;
    let mut index = repo.index().map_err(|e| e.to_string())?;

    if file_path == "--all" {
        // Stage modifications and deletions of tracked files.
        index
            .update_all(["*"].iter(), None)
            .map_err(|e| format!("index update failed: {}", e))?;
        // Stage any new untracked files.
        index
            .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
            .map_err(|e| format!("index add_all failed: {}", e))?;
    } else {
        let p = Path::new(file_path);
        if vault_path.join(file_path).exists() {
            index
                .add_path(p)
                .map_err(|e| format!("index add '{}' failed: {}", file_path, e))?;
        } else {
            // File was deleted — remove it from the index.
            index
                .remove_path(p)
                .map_err(|e| format!("index remove '{}' failed: {}", file_path, e))?;
        }
    }

    index.write().map_err(|e| e.to_string())?;
    let tree_id = index.write_tree().map_err(|e| e.to_string())?;

    // Nothing to commit when the new tree is identical to HEAD.
    if let Ok(head) = repo.head() {
        if let Ok(head_commit) = head.peel_to_commit() {
            if head_commit.tree_id() == tree_id {
                return Ok(());
            }
        }
    }

    let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;
    let sig = get_signature(&repo)?;
    let message = format!("Update {}", file_path);

    let head_commit: Option<git2::Commit<'_>> =
        repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = head_commit.iter().collect();

    repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &parents)
        .map_err(|e| format!("commit failed: {}", e))?;

    Ok(())
}

/// Returns up to 50 commits that touched `rel_path`, ordered newest-first.
/// Diffs each commit against its parent to determine whether the file was
/// modified in that revision.
pub fn get_note_history(vault_path: &Path, rel_path: &str) -> Result<Vec<CommitInfo>, String> {
    let repo = open_repo(vault_path)?;

    let head_oid = match repo.head() {
        Ok(h) => h.target().ok_or("HEAD has no target OID")?,
        Err(_) => return Ok(vec![]), // Repository has no commits yet.
    };

    let mut revwalk = repo.revwalk().map_err(|e| e.to_string())?;
    revwalk.push(head_oid).map_err(|e| e.to_string())?;
    revwalk
        .set_sorting(git2::Sort::TIME)
        .map_err(|e| e.to_string())?;

    let target = Path::new(rel_path);
    let mut results = Vec::new();

    'walk: for oid_result in revwalk {
        if results.len() >= 50 {
            break 'walk;
        }

        let oid = oid_result.map_err(|e| e.to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.to_string())?;
        let tree = commit.tree().map_err(|e| e.to_string())?;

        let touches = match commit.parent(0) {
            Ok(parent) => {
                let parent_tree = parent.tree().map_err(|e| e.to_string())?;
                let mut diffopts = git2::DiffOptions::new();
                diffopts.pathspec(rel_path);
                let diff = repo
                    .diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut diffopts))
                    .map_err(|e| e.to_string())?;
                diff.deltas().len() > 0
            }
            // Root commit: check that the file exists in the initial tree.
            Err(_) => tree.get_path(target).is_ok(),
        };

        if touches {
            let timestamp = chrono::Utc
                .timestamp_opt(commit.time().seconds(), 0)
                .single()
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();

            results.push(CommitInfo {
                hash: oid.to_string(),
                message: commit.summary().ok().flatten().unwrap_or("").to_string(),
                timestamp,
                author: commit.author().name().unwrap_or("").to_string(),
            });
        }
    }

    Ok(results)
}

/// Returns the raw UTF-8 content of `rel_path` as it existed at `commit_hash`.
pub fn get_note_at_commit(
    vault_path: &Path,
    commit_hash: &str,
    rel_path: &str,
) -> Result<String, String> {
    let repo = open_repo(vault_path)?;
    let oid = git2::Oid::from_str(commit_hash)
        .map_err(|e| format!("Invalid commit hash '{}': {}", commit_hash, e))?;
    let commit = repo
        .find_commit(oid)
        .map_err(|e| format!("Commit not found: {}", e))?;
    let tree = commit.tree().map_err(|e| e.to_string())?;
    let entry = tree.get_path(Path::new(rel_path)).map_err(|_| {
        format!("'{}' not found at commit {:.8}", rel_path, commit_hash)
    })?;
    let blob = repo.find_blob(entry.id()).map_err(|e| e.to_string())?;
    std::str::from_utf8(blob.content())
        .map(|s| s.to_string())
        .map_err(|e| format!("Note is not valid UTF-8: {}", e))
}

/// Fetch → integrate remote changes → push.
///
/// Integration strategy:
/// - Fast-forward when the local branch has not diverged from remote.
/// - Rebase when local and remote have diverged (maintains a linear history).
/// - Aborts and returns an error on conflicts.
///
/// `on_progress` is called at each stage with a human-readable stage name.
pub fn sync_vault(vault_path: &Path, on_progress: &dyn Fn(&str)) -> Result<GitResult, String> {
    let repo = open_repo(vault_path)?;
    let branch = current_branch(&repo)?;

    // ── Check for uncommitted changes ───────────────────────────────────────
    on_progress("Checking repository state...");
    let statuses = repo
        .statuses(None)
        .map_err(|e| format!("Failed to check repo status: {}", e))?;
    let has_dirty = statuses.iter().any(|s| {
        let flags = s.status();
        !flags.is_ignored()
            && (flags.contains(git2::Status::INDEX_NEW)
                || flags.contains(git2::Status::INDEX_MODIFIED)
                || flags.contains(git2::Status::INDEX_DELETED)
                || flags.contains(git2::Status::WT_NEW)
                || flags.contains(git2::Status::WT_MODIFIED)
                || flags.contains(git2::Status::WT_DELETED))
    });
    if has_dirty {
        return Err("Uncommitted changes detected. Auto-commit or stash before syncing.".to_string());
    }

    // ── Fetch ─────────────────────────────────────────────────────────────
    on_progress("Fetching from remote...");
    let mut remote = repo
        .find_remote("origin")
        .map_err(|e| format!("Remote 'origin' not found: {}", e))?;

    let mut cb = RemoteCallbacks::new();
    cb.credentials(credential_callback);
    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(cb);

    let branch_ref = format!("refs/heads/{}", branch);
    remote
        .fetch(&[&branch], Some(&mut fetch_opts), None)
        .map_err(|e| format!("Fetch failed: {}", e))?;
    drop(remote);

    // ── Integrate remote changes ───────────────────────────────────────────
    let fetch_head = repo
        .find_reference("FETCH_HEAD")
        .map_err(|_| "FETCH_HEAD not found — remote may have no commits yet".to_string())?;
    let fetch_commit = repo
        .reference_to_annotated_commit(&fetch_head)
        .map_err(|e| e.to_string())?;

    let (analysis, _) = repo
        .merge_analysis(&[&fetch_commit])
        .map_err(|e| e.to_string())?;

    if analysis.is_up_to_date() {
        on_progress("Already up to date.");
    } else if analysis.is_fast_forward() {
        on_progress("Fast-forwarding...");
        fast_forward(&repo, &fetch_commit)?;
    } else {
        on_progress("Rebasing local commits...");
        do_rebase(&repo, &fetch_commit)?;
    }

    // ── Push ──────────────────────────────────────────────────────────────
    on_progress("Pushing to remote...");
    let mut remote = repo.find_remote("origin").map_err(|e| e.to_string())?;
    let mut push_cb = RemoteCallbacks::new();
    push_cb.credentials(credential_callback);
    let mut push_opts = PushOptions::new();
    push_opts.remote_callbacks(push_cb);

    let push_spec = format!("{}:{}", branch_ref, branch_ref);
    remote
        .push(&[&push_spec], Some(&mut push_opts))
        .map_err(|e| format!("Push failed: {}", e))?;

    Ok(GitResult {
        success: true,
        output: "Sync complete".to_string(),
    })
}

// ── Private integration helpers ───────────────────────────────────────────────

fn fast_forward(repo: &Repository, onto: &AnnotatedCommit) -> Result<(), String> {
    let branch = current_branch(repo)?;
    let branch_ref_name = format!("refs/heads/{}", branch);
    let mut head_ref = repo
        .find_reference(&branch_ref_name)
        .map_err(|e| e.to_string())?;
    head_ref
        .set_target(onto.id(), "sync: fast-forward")
        .map_err(|e| e.to_string())?;
    repo.checkout_head(Some(CheckoutBuilder::default().force()))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Replays each local commit on top of `onto`.
/// Aborts and surfaces the conflict as an error on the first conflict.
fn do_rebase(repo: &Repository, onto: &AnnotatedCommit) -> Result<(), String> {
    let mut opts = git2::RebaseOptions::new();
    let mut rebase = repo
        .rebase(None, Some(onto), None, Some(&mut opts))
        .map_err(|e| format!("Rebase init failed: {}", e))?;

    loop {
        match rebase.next() {
            None => break,
            Some(Err(e)) => {
                let _ = rebase.abort();
                return Err(format!(
                    "Merge conflict detected. Sync aborted. Resolve manually. ({})",
                    e
                ));
            }
            Some(Ok(_op)) => {
                let index = repo.index().map_err(|e| e.to_string())?;
                if index.has_conflicts() {
                    let _ = rebase.abort();
                    return Err(
                        "Merge conflict detected. Sync aborted. Resolve manually.".to_string(),
                    );
                }
                let sig = get_signature(repo)?;
                rebase
                    .commit(None, &sig, None)
                    .map_err(|e| format!("Rebase commit failed: {}", e))?;
            }
        }
    }

    rebase
        .finish(None)
        .map_err(|e| format!("Rebase finish failed: {}", e))?;
    Ok(())
}