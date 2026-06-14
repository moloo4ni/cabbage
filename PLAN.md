# Cabbage Development Roadmap

## Phase 1 — Stability & Foundations

- [x] Auto-save debounce (1.5 s) with local git commit
- [x] Path traversal protection (`resolve_safe_path`)
- [x] Dead code removal (unused lock unwraps, `keyring`, vault module)
- [x] Git branch auto-detection, dirty-tree check before sync
- [x] Inline error bar for failed operations
- [x] Tauri v1 to v2 migration
- [x] CI: system deps for Tauri v2, ubuntu-24.04

## Phase 2 — Core Features

- [x] Full-text search (#14): fuzzy search across filenames + content, Ctrl+P hotkey, 20-result dropdown with snippets
- [x] Dark theme (GitHub Dark palette), `prefers-color-scheme` detection, toggle in Settings dropdown
- [x] Vault persistence (#15): last vault saved to app data dir, auto-reopens on launch
- [x] Incremental backlinks index (#6): re-scan only changed file instead of full tree
- [x] Pathspec-limited `get_note_history` diff (#7): O(commits x files) to O(commits x 1)
- [x] Loading indicators (#12): spinners for file open, tree refresh, graph load, create/delete

## Phase 3 — Power Features

- [ ] Note templates — create notes from user-defined templates with `{{date}}` placeholders
- [ ] Quick preview — hover or side-panel preview of a note without opening it
- [ ] Settings UI — persistent settings panel (vault path, theme, editor font size, line wrap)
- [ ] File attachments — copy/paste or drag files into vault
- [ ] Tag index — `#tag` parsing and navigation pane
- [ ] Export — single note or whole vault to HTML/PDF
- [ ] Plugin system (post-MVP) — WASM or Lua scripts for custom pipelines

## Future Ideas

- Graph: node filtering, minimap, search-to-locate
- Multi-vault: open multiple vaults, cross-vault search
- Mobile: Tauri mobile target for iOS/Android
- Auto-sync: periodic background sync with configurable interval
- Encryption: optional per-file or per-vault encryption via age/gpg
