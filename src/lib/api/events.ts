// src/lib/api/events.ts
// Tauri event listeners

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  ScanProgress,
  FileCarvedEvent,
  LogMessageEvent,
  ScanStateChangedEvent,
  ArtefactFoundEvent,
} from './types';

export interface ScanEventHandlers {
  onProgress?: (progress: ScanProgress) => void;
  onFileCarved?: (file: FileCarvedEvent) => void;
  onLog?: (log: LogMessageEvent) => void;
  onStateChange?: (state: ScanStateChangedEvent) => void;
  onArtefact?: (artefact: ArtefactFoundEvent) => void;
  onCompleted?: (summary: { run_id: string }) => void;
  onFailed?: (error: string) => void;
}

/**
 * Set up all scan event listeners
 * Returns a cleanup function to unsubscribe from all events
 */
export async function setupScanListeners(handlers: ScanEventHandlers): Promise<UnlistenFn> {
  const unlisteners: UnlistenFn[] = [];

  if (handlers.onProgress) {
    const unlisten = await listen<ScanProgress>('scan:progress', (event) => {
      handlers.onProgress!(event.payload);
    });
    unlisteners.push(unlisten);
  }

  if (handlers.onFileCarved) {
    const unlisten = await listen<FileCarvedEvent>('scan:file_carved', (event) => {
      handlers.onFileCarved!(event.payload);
    });
    unlisteners.push(unlisten);
  }

  if (handlers.onLog) {
    const unlisten = await listen<LogMessageEvent>('scan:log', (event) => {
      handlers.onLog!(event.payload);
    });
    unlisteners.push(unlisten);
  }

  if (handlers.onStateChange) {
    const unlisten = await listen<ScanStateChangedEvent>('scan:state', (event) => {
      handlers.onStateChange!(event.payload);
    });
    unlisteners.push(unlisten);
  }

  if (handlers.onArtefact) {
    const unlisten = await listen<ArtefactFoundEvent>('scan:artefact', (event) => {
      handlers.onArtefact!(event.payload);
    });
    unlisteners.push(unlisten);
  }

  if (handlers.onCompleted) {
    const unlisten = await listen<{ run_id: string }>('scan:completed', (event) => {
      handlers.onCompleted!(event.payload);
    });
    unlisteners.push(unlisten);
  }

  if (handlers.onFailed) {
    const unlisten = await listen<string>('scan:failed', (event) => {
      handlers.onFailed!(event.payload);
    });
    unlisteners.push(unlisten);
  }

  // Return a function that unsubscribes from all events
  return () => {
    unlisteners.forEach((unlisten) => unlisten());
  };
}

// =============================================================================
// Convenience functions for individual event types
// =============================================================================

interface SimpleScanEventHandlers {
  onProgress: (progress: ScanProgress) => void;
  onCompleted: (data: { run_id: string }) => void;
  onFailed: (error: string) => void;
}

/**
 * Listen to scan progress, completed, and failed events
 */
export async function listenToScanEvents(handlers: SimpleScanEventHandlers): Promise<UnlistenFn> {
  const unlisteners: UnlistenFn[] = [];

  const progressUnlisten = await listen<ScanProgress>('scan:progress', (event) => {
    handlers.onProgress(event.payload);
  });
  unlisteners.push(progressUnlisten);

  const completedUnlisten = await listen<{ run_id: string }>('scan:completed', (event) => {
    handlers.onCompleted(event.payload);
  });
  unlisteners.push(completedUnlisten);

  const failedUnlisten = await listen<string>('scan:failed', (event) => {
    handlers.onFailed(event.payload);
  });
  unlisteners.push(failedUnlisten);

  return () => {
    unlisteners.forEach((unlisten) => unlisten());
  };
}

/**
 * Listen to scan state change events
 */
export async function listenToStateEvents(
  handler: (state: ScanStateChangedEvent) => void
): Promise<UnlistenFn> {
  return listen<ScanStateChangedEvent>('scan:state', (event) => {
    handler(event.payload);
  });
}

/**
 * Listen to log events
 */
export async function listenToLogEvents(
  handler: (log: LogMessageEvent) => void
): Promise<UnlistenFn> {
  return listen<LogMessageEvent>('scan:log', (event) => {
    handler(event.payload);
  });
}

/**
 * Listen to file carved events
 */
export async function listenToFileCarvedEvents(
  handler: (file: FileCarvedEvent) => void
): Promise<UnlistenFn> {
  return listen<FileCarvedEvent>('scan:file_carved', (event) => {
    handler(event.payload);
  });
}

/**
 * Listen to artefact found events
 */
export async function listenToArtefactEvents(
  handler: (artefact: ArtefactFoundEvent) => void
): Promise<UnlistenFn> {
  return listen<ArtefactFoundEvent>('scan:artefact', (event) => {
    handler(event.payload);
  });
}
