use std::path::Path;
use walkdir::WalkDir;

#[derive(serde::Serialize)]
pub struct SearchResult {
    pub path: String,
    pub name: String,
    pub snippet: String,
}

/// Searches .md files in the vault by filename and content.
/// Returns up to 20 results, prioritizing filename matches.
pub fn search_notes(vault_root: &Path, query: &str) -> Vec<SearchResult> {
    let q = query.to_lowercase();
    let mut results = Vec::new();

    for entry in WalkDir::new(vault_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_type().is_file()
                && e.path().extension().is_some_and(|ext| ext == "md")
                && !e.path().components().any(|c| {
                    c.as_os_str().to_string_lossy().starts_with('.')
                })
        })
    {
        let rel = entry
            .path()
            .strip_prefix(vault_root)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .to_string();

        let name = entry.file_name().to_string_lossy().to_string();
        let name_lower = name.to_lowercase();

        let is_filename_match = name_lower.contains(&q);

        let content = if !is_filename_match {
            std::fs::read_to_string(entry.path()).unwrap_or_default()
        } else {
            String::new()
        };

        let snippet = if is_filename_match {
            name.clone()
        } else {
            let content_lower = content.to_lowercase();
            if let Some(pos) = content_lower.find(&q) {
                let start = pos.saturating_sub(40);
                let end = (pos + q.len() + 80).min(content.len());
                let mut s = content[start..end].to_string();
                if start > 0 {
                    s = format!("…{}", s);
                }
                if end < content.len() {
                    s = format!("{}…", s);
                }
                s
            } else {
                continue;
            }
        };

        // Prioritize filename matches
        if is_filename_match {
            results.insert(0, SearchResult { path: rel, name, snippet });
        } else {
            results.push(SearchResult { path: rel, name, snippet });
        }

        if results.len() >= 20 {
            break;
        }
    }

    results
}
