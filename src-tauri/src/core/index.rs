use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;
use regex::Regex;

fn wiki_link_re() -> &'static Regex {
    static RE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\[\[([^\[\]]+)\]\]").unwrap());
    &RE
}

/// Scans all .md files in the vault and builds a backlinks index.
/// backlinks[target] = Vec<source> — which notes link TO `target`.
pub fn build_index(vault_path: &Path) -> HashMap<String, Vec<String>> {
    let re = wiki_link_re();
    let mut backlinks: HashMap<String, Vec<String>> = HashMap::new();

    for entry in WalkDir::new(vault_path)
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
        let source = entry
            .path()
            .strip_prefix(vault_path)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .to_string();

        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for cap in re.captures_iter(&content) {
            let target = cap[1].trim().to_string();
            backlinks
                .entry(target)
                .or_default()
                .push(source.clone());
        }
    }

    backlinks
}

/// Re-scans a single `.md` file and updates the backlinks index incrementally.
/// Removes any old links originating from `rel_path`, then inserts new ones.
pub fn update_index(
    backlinks: &mut HashMap<String, Vec<String>>,
    vault_path: &Path,
    rel_path: &str,
) {
    // Remove all existing backlinks originating from this file.
    for sources in backlinks.values_mut() {
        sources.retain(|s| s != rel_path);
    }
    // Remove empty entries left by the removal above.
    backlinks.retain(|_, v| !v.is_empty());

    // Scan the file and re-insert its links.
    let abs_path = vault_path.join(rel_path);
    if !abs_path.exists() {
        return;
    }
    let content = match std::fs::read_to_string(&abs_path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let re = wiki_link_re();
    for cap in re.captures_iter(&content) {
        let target = cap[1].trim().to_string();
        backlinks
            .entry(target)
            .or_default()
            .push(rel_path.to_string());
    }
}

/// Removes all backlinks originating from `rel_path` (e.g. when a note is deleted).
pub fn remove_file(
    backlinks: &mut HashMap<String, Vec<String>>,
    rel_path: &str,
) {
    backlinks.values_mut().for_each(|sources| sources.retain(|s| s != rel_path));
    backlinks.retain(|_, v| !v.is_empty());
}
