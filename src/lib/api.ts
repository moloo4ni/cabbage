import { invoke } from '@tauri-apps/api/core';

export interface FileNode {
    name: string;
    path: string;
    is_dir: boolean;
}

export interface GitResult {
    success: boolean;
    output: string;
}

export interface CommitInfo {
    hash: string;
    message: string;
    timestamp: string;
    author: string;
}

export interface GraphNode {
    id: string;
}

export interface GraphEdge {
    source: string;
    target: string;
}

export interface GraphData {
    nodes: GraphNode[];
    edges: GraphEdge[];
}

export interface SearchResult {
    path: string;
    name: string;
    snippet: string;
}

export const api = {
    // ── Vault ──────────────────────────────────────────────────────────────

    /** Opens an OS folder-picker dialog and sets the active vault. */
    async pickAndOpenVault(): Promise<string> {
        return invoke('pick_and_open_vault');
    },

    /** Opens a vault at a known path (e.g. recently used). */
    async openVault(path: string): Promise<string> {
        return invoke('open_vault', { path });
    },

    /** Returns the path of the last opened vault, if any. */
    async getLastVault(): Promise<string | null> {
        return invoke('get_last_vault');
    },

    // ── File tree ──────────────────────────────────────────────────────────

    async listDirectory(subPath: string = ''): Promise<FileNode[]> {
        return invoke('list_directory', { subPath });
    },

    // ── Search ─────────────────────────────────────────────────────────────

    async searchNotes(query: string): Promise<SearchResult[]> {
        return invoke('search_notes', { query });
    },

    // ── Note CRUD ──────────────────────────────────────────────────────────

    async readNote(relPath: string): Promise<string> {
        return invoke('read_note', { relPath });
    },

    async writeNote(relPath: string, content: string): Promise<void> {
        return invoke('write_note', { relPath, content });
    },

    async createNote(relPath: string): Promise<void> {
        return invoke('create_note', { relPath });
    },

    async deleteNote(relPath: string): Promise<void> {
        return invoke('delete_note', { relPath });
    },

    // ── Knowledge graph ────────────────────────────────────────────────────

    /** Returns paths of all notes that link to `noteName` via [[noteName]]. */
    async getBacklinks(noteName: string): Promise<string[]> {
        return invoke('get_backlinks', { noteName });
    },

    /** Returns all notes as graph nodes and all [[wikilink]] edges between them. */
    async getGraph(): Promise<GraphData> {
        return invoke('get_graph');
    },

    // ── Git sync ───────────────────────────────────────────────────────────

    async sync(): Promise<GitResult> {
        return invoke('sync');
    },

    // ── Note history ───────────────────────────────────────────────────────

    /** Returns up to 50 git commits for the given note file. */
    async getNoteHistory(relPath: string): Promise<CommitInfo[]> {
        return invoke('get_note_history', { relPath });
    },

    /** Returns the raw content of a note at a specific commit (preview only). */
    async getNoteAtCommit(relPath: string, commitHash: string): Promise<string> {
        return invoke('get_note_at_commit', { relPath, commitHash });
    },

    /** Restores a note to a previous commit. Writes + auto-commits. Returns new content. */
    async restoreNoteVersion(relPath: string, commitHash: string): Promise<string> {
        return invoke('restore_note_version', { relPath, commitHash });
    },
};
