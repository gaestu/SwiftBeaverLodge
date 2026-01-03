// src/lib/stores/scan.svelte.ts
// Scan state management using Svelte 5 runes

import type {
  ScanState,
  ScanProgress,
  ScanConfig,
  FileCarvedEvent,
  LogMessageEvent,
  DEFAULT_SCAN_CONFIG,
} from '$lib/api/types';

// =============================================================================
// Scan Store State
// =============================================================================

interface ScanStoreState {
  status: ScanState;
  runId: string | null;
  startedAt: Date | null;
  progress: ScanProgress | null;
  error: string | null;
  carvedFiles: FileCarvedEvent[];
  logs: LogMessageEvent[];
  fileCountsByType: Record<string, number>;
}

function createScanStore() {
  let state = $state<ScanStoreState>({
    status: 'idle',
    runId: null,
    startedAt: null,
    progress: null,
    error: null,
    carvedFiles: [],
    logs: [],
    fileCountsByType: {},
  });

  // Derived values
  const percentage = $derived(
    state.progress && state.progress.total_bytes > 0
      ? (state.progress.bytes_scanned / state.progress.total_bytes) * 100
      : 0
  );

  const isRunning = $derived(state.status === 'running');
  const isPaused = $derived(state.status === 'paused');
  const isIdle = $derived(state.status === 'idle');
  const isFinished = $derived(
    state.status === 'completed' || state.status === 'failed' || state.status === 'cancelled'
  );

  const totalFilesCarved = $derived(
    Object.values(state.fileCountsByType).reduce((sum, count) => sum + count, 0)
  );

  return {
    // Getters
    get status() { return state.status; },
    get runId() { return state.runId; },
    get startedAt() { return state.startedAt; },
    get progress() { return state.progress; },
    get error() { return state.error; },
    get carvedFiles() { return state.carvedFiles; },
    get logs() { return state.logs; },
    get fileCountsByType() { return state.fileCountsByType; },
    get percentage() { return percentage; },
    get isRunning() { return isRunning; },
    get isPaused() { return isPaused; },
    get isIdle() { return isIdle; },
    get isFinished() { return isFinished; },
    get totalFilesCarved() { return totalFilesCarved; },

    // Actions
    start(runId: string) {
      state.status = 'running';
      state.runId = runId;
      state.startedAt = new Date();
      state.error = null;
      state.carvedFiles = [];
      state.logs = [];
      state.fileCountsByType = {};
      state.progress = null;
    },

    updateProgress(progress: ScanProgress) {
      state.progress = progress;
    },

    addCarvedFile(file: FileCarvedEvent) {
      state.carvedFiles = [...state.carvedFiles.slice(-99), file]; // Keep last 100
      state.fileCountsByType = {
        ...state.fileCountsByType,
        [file.file_type]: (state.fileCountsByType[file.file_type] || 0) + 1,
      };
    },

    addLog(log: LogMessageEvent) {
      state.logs = [...state.logs.slice(-499), log]; // Keep last 500
    },

    pause() {
      if (state.status === 'running') {
        state.status = 'paused';
      }
    },

    resume() {
      if (state.status === 'paused') {
        state.status = 'running';
      }
    },

    complete() {
      state.status = 'completed';
    },

    fail(error: string) {
      state.status = 'failed';
      state.error = error;
    },

    cancel() {
      state.status = 'cancelled';
    },

    reset() {
      state.status = 'idle';
      state.runId = null;
      state.startedAt = null;
      state.progress = null;
      state.error = null;
      state.carvedFiles = [];
      state.logs = [];
      state.fileCountsByType = {};
    },
  };
}

export const scanStore = createScanStore();
