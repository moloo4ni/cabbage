# Cabbage

Cabbage is a local-first, cross-platform desktop application for personal knowledge management. It stores all notes as plain Markdown files inside a Git repository, giving you versioning, offline use, and full data portability — no proprietary database, no cloud account, no vendor lock-in.

## How It Works

A **vault** is simply a local directory on your file system. Open any folder in Cabbage and it becomes your vault — if it is not already a Git repository, Cabbage runs `git init` automatically.

Notes are standard `.md` files. As you edit, Cabbage saves your changes and silently commits them to the local repository. Syncing with another machine is a regular `git push` and `git pull --rebase` against any remote you configure (GitHub, GitLab, a self-hosted server — anything that speaks Git over SSH or HTTPS).

## Current State

The core read/write/sync loop is working:

- Open a vault via a native folder-picker dialog — last vault is persisted and reopens automatically on launch
- Browse the file tree in the sidebar
- Create and delete notes (with loading indicators)
- Edit notes with CodeMirror 6 — Markdown syntax highlighting, line wrapping, minimal theme
- `[[wiki-links]]` highlighted inline; Ctrl/Cmd+click navigates to the linked note (creates it if it does not exist)
- Auto-save with a 1.5 s debounce — every save triggers an automatic local Git commit
- Full-text search — Ctrl+P / Cmd+P opens a fuzzy search box that matches filenames (priority) and content with snippets (up to 20 results)
- Graph view — canvas force-directed graph of all notes and `[[wikilink]]` connections
- Sync button runs fetch → fast-forward or rebase → push, all via native libgit2 (no `git` binary required) with progress events
- Incremental backlinks index — notes that link to the current note are shown via per-file scan (no full tree rebuild on every change)
- History panel — browse up to 50 commits for the active note, preview any version, restore with one click
- Dark theme — GitHub Dark palette, auto-detects `prefers-color-scheme`, toggleable from Settings dropdown
- Loading states — spinners for file open, file tree refresh, graph load, note create/delete
- Inline error bar for failed operations

## Architecture

The application is structured as a decoupled system:

- **Frontend (Svelte 4):** Handles UI rendering and user interactions. Holds no persistent state — everything is fetched from the Rust core via IPC. Editor is CodeMirror 6 with a custom `[[wiki-link]]` extension. Icons from `lucide-svelte`.
- **Bridge (Tauri v2 IPC):** Secure communication channel between the Svelte webview and the native system.
- **Core (Rust):** File system operations, native Git operations via `git2` (libgit2 bindings), and full-text search via file-tree walk on query.

## Roadmap

See [PLAN.md](PLAN.md) for the full 3-phase roadmap.

Implemented highlights:
- [x] CodeMirror 6 editor with Markdown syntax highlighting
- [x] `[[wiki-link]]` highlighting and click-to-navigate
- [x] Note history view (per-file `git log` + version preview + restore)
- [x] Graph view — canvas force-directed graph of all notes and wikilink connections
- [x] Native Rust Git bindings — `git2` / libgit2, no system `git` binary required
- [x] Full-text search across all notes with fuzzy matching
- [x] Dark theme (GitHub Dark palette)
- [x] Vault persistence — last vault reopens on launch
- [x] Incremental backlinks index
- [x] Pathspec-optimized note history diff
- [x] Loading indicators for slow operations

Remaining:
- [ ] Note templates
- [ ] Quick preview
- [ ] Settings UI
- [ ] File attachments
- [ ] Tag index
- [ ] Export

## Local Development

**Prerequisites:** Rust toolchain, Node.js (v22+), pnpm, Git, and the [Tauri v2 system dependencies](https://v2.tauri.app/start/prerequisites/) for your OS.

```bash
git clone https://github.com/moloo4ni/cabbage.git
cd cabbage
pnpm install
pnpm tauri dev
```

**Build a release binary:**

```bash
pnpm tauri build
# Output: src-tauri/target/release/bundle/
```

## Project Status

All **16 issues** are closed. The project is stable with core functionality complete. Future work is tracked in [PLAN.md](PLAN.md) Phase 3 and ideas.md.

## Disclaimer

Cabbage does not track user metrics, require registration, or communicate with any external servers other than the Git remotes you configure yourself. Everything runs locally on your machine.
