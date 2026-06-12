use std::path::PathBuf;
use std::collections::HashSet;
use tauri::State;
use crate::state::AppState;
use crate::core::{fs, index};
use crate::git::cli;

// ── Vault ─────────────────────────────────────────────────────────────────────

/// Opens a folder-picker dialog and sets the active vault.
/// Ensures the selected directory is a git repository (init if needed).
#[tauri::command]
pub async fn pick_and_open_vault(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    use tauri::api::dialog::blocking::FileDialogBuilder;

    let path = FileDialogBuilder::new()
        .set_title("Open Vault")
        .pick_folder()
        .ok_or("No folder selected")?;

    open_vault_path(app, state, path.to_string_lossy().to_string()).await
}

/// Opens a vault at a specific path (useful for reopening recent vaults).
#[tauri::command]
pub async fn open_vault(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    open_vault_path(app, state, path).await
}

async fn open_vault_path(
    _app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    let vault_path = PathBuf::from(&path);

    if !vault_path.exists() || !vault_path.is_dir() {
        return Err("Invalid vault path".into());
    }

    cli::ensure_git_repo(&vault_path)?;

    let backlinks = index::build_index(&vault_path);
    *state.current_vault.lock().map_err(|e| e.to_string())? = Some(vault_path.clone());
    *state.backlinks.lock().map_err(|e| e.to_string())? = backlinks;

    Ok(path)
}

// ── File tree ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_directory(
    state: State<'_, AppState>,
    sub_path: String,
) -> Result<Vec<fs::FileNode>, String> {
    let lock = state.current_vault.lock().map_err(|e| e.to_string())?;
    let root = lock.as_ref().ok_or("Vault not opened")?;
    fs::list_directory(root, &sub_path)
}

// ── Note CRUD ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn read_note(state: State<'_, AppState>, rel_path: String) -> Result<String, String> {
    let lock = state.current_vault.lock().map_err(|e| e.to_string())?;
    let root = lock.as_ref().ok_or("Vault not opened")?;
    fs::read_note(root, &rel_path)
}

#[tauri::command]
pub fn write_note(
    state: State<'_, AppState>,
    rel_path: String,
    content: String,
) -> Result<(), String> {
    let lock = state.current_vault.lock().map_err(|e| e.to_string())?;
    let root = lock.as_ref().ok_or("Vault not opened")?;
    let root = root.clone();
    drop(lock);

    fs::write_note(&root, &rel_path, &content)?;
    cli::auto_commit(&root, &rel_path)?;

    // Rebuild backlinks index after save
    let fresh = index::build_index(&root);
    *state.backlinks.lock().map_err(|e| e.to_string())? = fresh;

    Ok(())
}

#[tauri::command]
pub fn create_note(
    state: State<'_, AppState>,
    rel_path: String,
) -> Result<(), String> {
    let lock = state.current_vault.lock().map_err(|e| e.to_string())?;
    let root = lock.as_ref().ok_or("Vault not opened")?;
    let root = root.clone();
    drop(lock);

    fs::create_note(&root, &rel_path)?;
    cli::auto_commit(&root, &rel_path)?;
    Ok(())
}

#[tauri::command]
pub fn delete_note(
    state: State<'_, AppState>,
    rel_path: String,
) -> Result<(), String> {
    let lock = state.current_vault.lock().map_err(|e| e.to_string())?;
    let root = lock.as_ref().ok_or("Vault not opened")?;
    let root = root.clone();
    drop(lock);

    fs::delete_note(&root, &rel_path)?;
    cli::auto_commit(&root, "--all")?;

    let fresh = index::build_index(&root);
    *state.backlinks.lock().map_err(|e| e.to_string())? = fresh;

    Ok(())
}

// ── Backlinks ────────────────────────────────────────────────────────────────

/// Returns the list of notes that contain a [[link]] pointing to `note_name`.
#[tauri::command]
pub fn get_backlinks(
    state: State<'_, AppState>,
    note_name: String,
) -> Result<Vec<String>, String> {
    let lock = state.backlinks.lock().map_err(|e| e.to_string())?;
    Ok(lock.get(&note_name).cloned().unwrap_or_default())
}

// ── Git sync ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn sync(state: State<'_, AppState>) -> Result<cli::GitResult, String> {
    let lock = state.current_vault.lock().map_err(|e| e.to_string())?;
    let root = lock.as_ref().ok_or("Vault not opened")?;
    let root = root.clone();
    drop(lock);

    cli::sync_vault(&root)
}

// ── Note history ──────────────────────────────────────────────────────────────

/// Returns the git commit history for a single note (up to 50 entries).
#[tauri::command]
pub fn get_note_history(
    state: State<'_, AppState>,
    rel_path: String,
) -> Result<Vec<cli::CommitInfo>, String> {
    let lock = state.current_vault.lock().map_err(|e| e.to_string())?;
    let root = lock.as_ref().ok_or("Vault not opened")?;
    cli::get_note_history(root, &rel_path)
}

/// Returns the content of a note at a specific commit (read-only preview).
#[tauri::command]
pub fn get_note_at_commit(
    state: State<'_, AppState>,
    rel_path: String,
    commit_hash: String,
) -> Result<String, String> {
    let lock = state.current_vault.lock().map_err(|e| e.to_string())?;
    let root = lock.as_ref().ok_or("Vault not opened")?;
    cli::get_note_at_commit(root, &commit_hash, &rel_path)
}

/// Restores a note to a previous version: writes the old content and auto-commits.
/// Returns the restored content so the frontend can update the editor immediately.
#[tauri::command]
pub fn restore_note_version(
    state: State<'_, AppState>,
    rel_path: String,
    commit_hash: String,
) -> Result<String, String> {
    let lock = state.current_vault.lock().map_err(|e| e.to_string())?;
    let root = lock.as_ref().ok_or("Vault not opened")?;
    let root = root.clone();
    drop(lock);

    let content = cli::get_note_at_commit(&root, &commit_hash, &rel_path)?;
    fs::write_note(&root, &rel_path, &content)?;
    cli::auto_commit(&root, &rel_path)?;

    let fresh = index::build_index(&root);
    *state.backlinks.lock().map_err(|e| e.to_string())? = fresh;

    Ok(content)
}

// ── Graph view ────────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct GraphNode {
    pub id: String,
}

#[derive(serde::Serialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
}

#[derive(serde::Serialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// Returns all notes as nodes and all [[wikilink]] connections as edges.
/// Nodes include every .md file in the vault plus any unresolved link targets.
#[tauri::command]
pub fn get_graph(state: State<'_, AppState>) -> Result<GraphData, String> {
    use walkdir::WalkDir;

    let vault_lock = state.current_vault.lock().map_err(|e| e.to_string())?;
    let root = vault_lock.as_ref().ok_or("Vault not opened")?;
    let root = root.clone();
    drop(vault_lock);

    // Seed node set with every .md file currently on disk
    let mut node_ids: HashSet<String> = HashSet::new();
    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_type().is_file()
                && e.path().extension().is_some_and(|ext| ext == "md")
                && !e
                    .path()
                    .components()
                    .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
        })
    {
        if let Some(stem) = entry.path().file_stem() {
            node_ids.insert(stem.to_string_lossy().to_string());
        }
    }

    // Build directed edges from the backlinks index.
    // backlinks[target] = Vec<source_rel_path>  =>  edge: source_name -> target
    let bl_lock = state.backlinks.lock().map_err(|e| e.to_string())?;
    let mut edges: Vec<GraphEdge> = Vec::new();

    for (target, sources) in bl_lock.iter() {
        // An unresolved target (note not yet created) still becomes a node
        node_ids.insert(target.clone());

        for source_path in sources {
            let source_name = std::path::Path::new(source_path)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            if source_name.is_empty() {
                continue;
            }

            node_ids.insert(source_name.clone());
            edges.push(GraphEdge {
                source: source_name,
                target: target.clone(),
            });
        }
    }
    drop(bl_lock);

    let nodes = node_ids
        .into_iter()
        .map(|id| GraphNode { id })
        .collect();

    Ok(GraphData { nodes, edges })
}
