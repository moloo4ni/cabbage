import { writable } from 'svelte/store';
import type { FileNode } from './api';

function createTheme() {
  const stored = typeof localStorage !== 'undefined' && localStorage.getItem('cabbage-theme');
  const preferDark = typeof window !== 'undefined' && window.matchMedia('(prefers-color-scheme: dark)').matches;
  const initial = stored || (preferDark ? 'dark' : 'light');
  const { subscribe, set, update } = writable<string>(initial);
  return {
    subscribe,
    toggle: () => {
      update((current: string) => {
        const next = current === 'dark' ? 'light' : 'dark';
        localStorage.setItem('cabbage-theme', next);
        document.documentElement.classList.toggle('dark', next === 'dark');
        return next;
      });
    },
    init: () => {
      const v = initial;
      document.documentElement.classList.toggle('dark', v === 'dark');
    },
  };
}

export const theme = createTheme();

/** Absolute path to the currently open vault directory. */
export const activeVault = writable<string | null>(null);

/** Vault-relative path of the note currently open in the editor. */
export const activeNotePath = writable<string | null>(null);

/** Flat list of file/folder nodes shown in the sidebar. */
export const fileTree = writable<FileNode[]>([]);

/** True while a git sync operation is in progress. */
export const isSyncing = writable<boolean>(false);

/** Paths of notes that link to the currently active note (backlinks panel). */
export const backlinks = writable<string[]>([]);
