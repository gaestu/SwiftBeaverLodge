// src/lib/stores/config.svelte.ts
// Configuration state management

import { DEFAULT_SCAN_CONFIG, type ScanConfig } from '$lib/api/types';

function createConfigStore() {
  let config = $state<ScanConfig>({ ...DEFAULT_SCAN_CONFIG });
  let isDirty = $state(false);
  let lastSavedPath = $state<string | null>(null);

  return {
    // Getters
    get config() { return config; },
    get isDirty() { return isDirty; },
    get lastSavedPath() { return lastSavedPath; },

    // Actions
    setConfig(newConfig: ScanConfig) {
      config = { ...newConfig };
      isDirty = false;
    },

    update(newConfig: Partial<ScanConfig>) {
      config = { ...config, ...newConfig };
      isDirty = true;
    },

    updateField<K extends keyof ScanConfig>(field: K, value: ScanConfig[K]) {
      config = { ...config, [field]: value };
      isDirty = true;
    },

    toggleFileType(fileType: string) {
      const types = config.file_types;
      if (types.includes(fileType)) {
        config = { ...config, file_types: types.filter(t => t !== fileType) };
      } else {
        config = { ...config, file_types: [...types, fileType] };
      }
      isDirty = true;
    },

    selectAllFileTypes(allTypes: string[]) {
      config = { ...config, file_types: [...allTypes] };
      isDirty = true;
    },

    clearAllFileTypes() {
      config = { ...config, file_types: [] };
      isDirty = true;
    },

    markSaved(path: string) {
      isDirty = false;
      lastSavedPath = path;
    },

    reset() {
      config = { ...DEFAULT_SCAN_CONFIG };
      isDirty = false;
      lastSavedPath = null;
    },
  };
}

export const configStore = createConfigStore();
