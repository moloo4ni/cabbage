<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type CommitInfo } from './api';
  import { RotateCcw } from '@lucide/svelte';

  export let relPath: string;
  export let onRestore: (content: string) => void;

  let commits: CommitInfo[] = [];
  let selected: CommitInfo | null = null;
  let preview: string = '';
  let loading = true;
  let previewLoading = false;
  let error = '';
  let restoring = false;

  onMount(async () => {
    await loadHistory();
  });

  // Reload when the active note changes
  $: if (relPath) loadHistory();

  async function loadHistory() {
    loading = true;
    error = '';
    selected = null;
    preview = '';
    try {
      commits = await api.getNoteHistory(relPath);
    } catch (e) {
      error = `Failed to load history: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function selectCommit(commit: CommitInfo) {
    selected = commit;
    previewLoading = true;
    try {
      preview = await api.getNoteAtCommit(relPath, commit.hash);
    } catch (e) {
      preview = '';
      error = `Failed to load version: ${e}`;
    } finally {
      previewLoading = false;
    }
  }

  async function restore() {
    if (!selected) return;
    if (!confirm(`Restore to "${selected.message}"?\n\nThis will overwrite the current content and create a new commit.`)) return;
    restoring = true;
    try {
      const content = await api.restoreNoteVersion(relPath, selected.hash);
      onRestore(content);
    } catch (e) {
      error = `Restore failed: ${e}`;
    } finally {
      restoring = false;
    }
  }

  function formatDate(iso: string): string {
    if (!iso) return '';
    const d = new Date(iso);
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
      + ' ' + d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
  }

  function shortHash(hash: string): string {
    return hash.slice(0, 7);
  }
</script>

<div class="history-panel">
  <div class="history-header">
    <span class="history-title">history</span>
    {#if selected}
      <button
        class="restore-btn"
        on:click={restore}
        disabled={restoring}
      >
        <RotateCcw size={14} /> {restoring ? 'restoring…' : 'restore'}
      </button>
    {/if}
  </div>

  {#if error}
    <div class="history-error">{error}</div>
  {/if}

  <div class="history-body">
    <!-- Commit list -->
    <div class="commit-list">
      {#if loading}
        <div class="history-hint">loading…</div>
      {:else if commits.length === 0}
        <div class="history-hint">no commits yet</div>
      {:else}
        {#each commits as commit}
          <!-- svelte-ignore a11y-click-events-have-key-events -->
          <!-- svelte-ignore a11y-no-static-element-interactions -->
          <div
            class="commit-row {selected?.hash === commit.hash ? 'active' : ''}"
            on:click={() => selectCommit(commit)}
          >
            <span class="commit-hash">{shortHash(commit.hash)}</span>
            <span class="commit-msg">{commit.message}</span>
            <span class="commit-date">{formatDate(commit.timestamp)}</span>
          </div>
        {/each}
      {/if}
    </div>

    <!-- Preview pane -->
    {#if selected}
      <div class="preview-pane">
        {#if previewLoading}
          <div class="history-hint">loading…</div>
        {:else}
          <pre class="preview-content">{preview}</pre>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .history-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    border-left: 1px solid var(--border-color);
    background: var(--sidebar-bg);
    min-width: 0;
  }

  .history-header {
    height: 44px;
    padding: 0 16px;
    border-bottom: 1px solid var(--border-color);
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;
  }
  .history-title {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    font-weight: 600;
  }

  .restore-btn {
    background: var(--accent);
    color: white;
    border: none;
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .restore-btn:hover:not(:disabled) { opacity: 0.85; }
  .restore-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .history-error {
    font-size: 12px;
    color: #991b1b;
    padding: 8px 16px;
    background: #fee2e2;
    flex-shrink: 0;
  }

  .history-body {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  .commit-list {
    overflow-y: auto;
    flex-shrink: 0;
    max-height: 260px;
    border-bottom: 1px solid var(--border-color);
  }

  .history-hint {
    padding: 16px;
    font-size: 13px;
    color: var(--text-muted);
  }

  .commit-row {
    padding: 8px 16px;
    cursor: pointer;
    border-bottom: 1px solid var(--border-color);
    display: grid;
    grid-template-columns: 52px 1fr;
    grid-template-rows: auto auto;
    gap: 2px 8px;
  }
  .commit-row:hover { background: rgba(0,0,0,0.04); }
  .commit-row.active { background: var(--border-color); }

  .commit-hash {
    font-family: monospace;
    font-size: 11px;
    color: var(--accent);
    grid-row: 1;
    grid-column: 1;
    align-self: center;
  }
  .commit-msg {
    font-size: 13px;
    color: var(--text-main);
    grid-row: 1;
    grid-column: 2;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .commit-date {
    font-size: 11px;
    color: var(--text-muted);
    grid-row: 2;
    grid-column: 2;
  }

  .preview-pane {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }
  .preview-content {
    margin: 0;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 13px;
    line-height: 1.6;
    color: var(--text-main);
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
