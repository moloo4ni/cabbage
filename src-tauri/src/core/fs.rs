use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(serde::Serialize)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

/// Resolves `rel_path` inside `vault_root` with path traversal protection.
/// Returns the canonicalized path for existing files; for new files,
/// normalises the path and verifies it stays inside the vault.
pub fn resolve_safe_path(vault_root: &Path, rel_path: &str) -> Result<PathBuf, String> {
    let root = vault_root
        .canonicalize()
        .map_err(|_| "Invalid vault path".to_string())?;
    let joined = root.join(rel_path);

    // Normalise path components (resolve .. and .)
    let mut normal = PathBuf::new();
    for c in joined.components() {
        match c {
            Component::ParentDir => {
                normal.pop();
            }
            Component::CurDir => {}
            other => normal.push(other.as_os_str()),
        }
    }

    // For existing files, verify with canonicalize
    if joined.exists() {
        let canonical = joined.canonicalize().map_err(|e| e.to_string())?;
        if canonical.starts_with(&root) {
            Ok(canonical)
        } else {
            Err("Path traversal detected".into())
        }
    } else if normal.starts_with(&root) {
        Ok(joined)
    } else {
        Err("Path traversal detected".into())
    }
}

pub fn list_directory(vault_root: &Path, sub_path: &str) -> Result<Vec<FileNode>, String> {
    let root = vault_root
        .canonicalize()
        .map_err(|_| "Invalid vault path".to_string())?;
    let target_dir: PathBuf = if sub_path.is_empty() {
        root.clone()
    } else {
        resolve_safe_path(&root, sub_path)?
    };

    let mut nodes = Vec::new();
    let entries = fs::read_dir(&target_dir).map_err(|e| e.to_string())?;

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') {
            continue;
        }

        let is_dir = path.is_dir();
        let rel_path = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");

        nodes.push(FileNode {
            name,
            path: rel_path,
            is_dir,
        });
    }

    nodes.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(nodes)
}

pub fn read_note(vault_root: &Path, rel_path: &str) -> Result<String, String> {
    let file_path = resolve_safe_path(vault_root, rel_path)?;
    fs::read_to_string(file_path).map_err(|e| e.to_string())
}

pub fn write_note(vault_root: &Path, rel_path: &str, content: &str) -> Result<(), String> {
    let file_path = resolve_safe_path(vault_root, rel_path)?;

    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    fs::write(file_path, content).map_err(|e| e.to_string())
}

pub fn create_note(vault_root: &Path, rel_path: &str) -> Result<(), String> {
    let file_path = resolve_safe_path(vault_root, rel_path)?;

    if file_path.exists() {
        return Err(format!("Note already exists: {}", rel_path));
    }

    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    fs::write(file_path, "").map_err(|e| e.to_string())
}

pub fn delete_note(vault_root: &Path, rel_path: &str) -> Result<(), String> {
    let file_path = resolve_safe_path(vault_root, rel_path)?;

    if !file_path.exists() {
        return Err(format!("Note not found: {}", rel_path));
    }

    fs::remove_file(file_path).map_err(|e| e.to_string())
}
