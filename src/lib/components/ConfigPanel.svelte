<script lang="ts">
  import { configStore, scanStore } from '$lib/stores';
  import { browseInputFile, browseOutputDir, startScan } from '$lib/api/commands';
  import { SUPPORTED_FILE_TYPES } from '$lib/api/types';
  import type { ScanConfig } from '$lib/api/types';

  // Local state for form
  let config = $state<ScanConfig>({ ...configStore.config });
  let isStarting = $state(false);
  let errorMessage = $state<string | null>(null);

  // File type categories
  const fileTypeCategories = {
    images: ['jpeg', 'png', 'gif', 'bmp', 'webp', 'tiff', 'ico', 'heic'],
    documents: ['pdf', 'docx', 'xlsx', 'pptx', 'rtf', 'odt'],
    archives: ['zip', 'rar', '7z', 'tar', 'gz'],
    databases: ['sqlite', 'mdb'],
    media: ['mp3', 'mp4', 'avi', 'mkv', 'wav', 'flac'],
    executables: ['exe', 'dll', 'elf', 'mach-o'],
    other: ['pst', 'eml', 'html', 'xml', 'json'],
  };

  async function handleBrowseInput() {
    try {
      const path = await browseInputFile();
      if (path) {
        config.input_path = path;
      }
    } catch (e) {
      errorMessage = `Failed to browse: ${e}`;
    }
  }

  async function handleBrowseOutput() {
    try {
      const path = await browseOutputDir();
      if (path) {
        config.output_path = path;
      }
    } catch (e) {
      errorMessage = `Failed to browse: ${e}`;
    }
  }

  function toggleFileType(fileType: string) {
    const index = config.file_types.indexOf(fileType);
    if (index >= 0) {
      config.file_types = config.file_types.filter(ft => ft !== fileType);
    } else {
      config.file_types = [...config.file_types, fileType];
    }
  }

  function selectAllInCategory(category: string[]) {
    const allSelected = category.every(ft => config.file_types.includes(ft));
    if (allSelected) {
      config.file_types = config.file_types.filter(ft => !category.includes(ft));
    } else {
      config.file_types = [...new Set([...config.file_types, ...category])];
    }
  }

  async function handleStartScan() {
    if (!config.input_path || !config.output_path) {
      errorMessage = 'Please select both input and output paths';
      return;
    }

    isStarting = true;
    errorMessage = null;

    try {
      configStore.update(config);
      const handle = await startScan(config);
      scanStore.start(handle.run_id);
    } catch (e) {
      errorMessage = `Failed to start scan: ${e}`;
    } finally {
      isStarting = false;
    }
  }

  // Derived: can start scan
  let canStart = $derived(
    !scanStore.isRunning && 
    config.input_path.length > 0 && 
    config.output_path.length > 0 &&
    config.file_types.length > 0
  );
</script>

<div class="max-w-4xl mx-auto space-y-6">
  <h2 class="text-2xl font-bold text-white">Scan Configuration</h2>

  {#if errorMessage}
    <div class="bg-red-900/50 border border-red-700 rounded-lg p-4 text-red-300">
      {errorMessage}
      <button 
        class="ml-4 text-red-400 hover:text-red-200"
        onclick={() => errorMessage = null}
      >
        ✕
      </button>
    </div>
  {/if}

  <!-- Input/Output Section -->
  <div class="card">
    <h3 class="card-header">📂 Evidence Source & Output</h3>
    
    <div class="space-y-4">
      <!-- Input Path -->
      <div>
        <label class="block text-sm font-medium text-slate-300 mb-2">
          Evidence File / Disk Image
        </label>
        <div class="flex gap-2">
          <input
            type="text"
            class="input flex-1"
            placeholder="/path/to/evidence.dd or /dev/sda"
            bind:value={config.input_path}
          />
          <button class="btn btn-secondary" onclick={handleBrowseInput}>
            Browse...
          </button>
        </div>
        <p class="text-xs text-slate-500 mt-1">
          Supports raw images (.dd, .raw, .img), E01 format, and block devices
        </p>
      </div>

      <!-- Output Path -->
      <div>
        <label class="block text-sm font-medium text-slate-300 mb-2">
          Output Directory
        </label>
        <div class="flex gap-2">
          <input
            type="text"
            class="input flex-1"
            placeholder="/path/to/output"
            bind:value={config.output_path}
          />
          <button class="btn btn-secondary" onclick={handleBrowseOutput}>
            Browse...
          </button>
        </div>
      </div>

      <!-- Evidence Hash -->
      <div class="flex items-center gap-4">
        <label class="flex items-center gap-2 cursor-pointer">
          <input
            type="checkbox"
            class="w-4 h-4 rounded bg-slate-700 border-slate-600 text-blue-600 focus:ring-blue-500"
            bind:checked={config.compute_evidence_hash}
          />
          <span class="text-sm text-slate-300">Compute evidence SHA-256 hash</span>
        </label>
      </div>
    </div>
  </div>

  <!-- File Types Section -->
  <div class="card">
    <h3 class="card-header">📁 File Types to Recover</h3>
    
    <div class="space-y-4">
      {#each Object.entries(fileTypeCategories) as [category, types]}
        <div>
          <div class="flex items-center justify-between mb-2">
            <span class="text-sm font-medium text-slate-400 capitalize">{category}</span>
            <button 
              class="text-xs text-blue-400 hover:text-blue-300"
              onclick={() => selectAllInCategory(types)}
            >
              {types.every(ft => config.file_types.includes(ft)) ? 'Deselect All' : 'Select All'}
            </button>
          </div>
          <div class="flex flex-wrap gap-2">
            {#each types as fileType}
              <button
                class="px-3 py-1 rounded-full text-sm transition-colors
                       {config.file_types.includes(fileType)
                         ? 'bg-blue-600 text-white'
                         : 'bg-slate-700 text-slate-300 hover:bg-slate-600'}"
                onclick={() => toggleFileType(fileType)}
              >
                {fileType.toUpperCase()}
              </button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </div>

  <!-- String Scanning Section -->
  <div class="card">
    <h3 class="card-header">🔍 String & Pattern Scanning</h3>
    
    <div class="grid grid-cols-2 gap-4">
      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" class="w-4 h-4 rounded" bind:checked={config.scan_strings} />
        <span class="text-sm text-slate-300">Extract text strings</span>
      </label>
      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" class="w-4 h-4 rounded" bind:checked={config.scan_urls} />
        <span class="text-sm text-slate-300">Extract URLs</span>
      </label>
      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" class="w-4 h-4 rounded" bind:checked={config.scan_emails} />
        <span class="text-sm text-slate-300">Extract email addresses</span>
      </label>
      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" class="w-4 h-4 rounded" bind:checked={config.scan_phones} />
        <span class="text-sm text-slate-300">Extract phone numbers</span>
      </label>
      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" class="w-4 h-4 rounded" bind:checked={config.scan_utf16} />
        <span class="text-sm text-slate-300">Scan UTF-16 strings</span>
      </label>
      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" class="w-4 h-4 rounded" bind:checked={config.scan_sqlite_pages} />
        <span class="text-sm text-slate-300">Recover SQLite pages</span>
      </label>
    </div>

    <div class="mt-4 grid grid-cols-2 gap-4">
      <div>
        <label class="block text-sm font-medium text-slate-400 mb-1">
          Min string length
        </label>
        <input
          type="number"
          class="input"
          min="4"
          max="256"
          bind:value={config.string_min_len}
        />
      </div>
      <div>
        <label class="block text-sm font-medium text-slate-400 mb-1">
          Max string length
        </label>
        <input
          type="number"
          class="input"
          min="64"
          max="65536"
          bind:value={config.string_max_len}
        />
      </div>
    </div>
  </div>

  <!-- Advanced Options -->
  <details class="card">
    <summary class="card-header cursor-pointer">⚙️ Advanced Options</summary>
    
    <div class="mt-4 space-y-4">
      <!-- GPU Acceleration -->
      <div>
        <label class="flex items-center gap-2 cursor-pointer mb-2">
          <input type="checkbox" class="w-4 h-4 rounded" bind:checked={config.gpu_enabled} />
          <span class="text-sm text-slate-300">Enable GPU acceleration</span>
        </label>
        {#if config.gpu_enabled}
          <select class="input w-48" bind:value={config.gpu_backend}>
            <option value="opencl">OpenCL</option>
            <option value="cuda">CUDA</option>
          </select>
        {/if}
      </div>

      <!-- Entropy Detection -->
      <div>
        <label class="flex items-center gap-2 cursor-pointer mb-2">
          <input type="checkbox" class="w-4 h-4 rounded" bind:checked={config.scan_entropy} />
          <span class="text-sm text-slate-300">Detect high-entropy regions (encrypted data)</span>
        </label>
        {#if config.scan_entropy}
          <div class="grid grid-cols-2 gap-4 mt-2">
            <div>
              <label class="block text-xs text-slate-500 mb-1">Entropy threshold</label>
              <input
                type="number"
                class="input"
                min="6"
                max="8"
                step="0.1"
                bind:value={config.entropy_threshold}
              />
            </div>
            <div>
              <label class="block text-xs text-slate-500 mb-1">Window size (bytes)</label>
              <input
                type="number"
                class="input"
                min="64"
                max="4096"
                bind:value={config.entropy_window_bytes}
              />
            </div>
          </div>
        {/if}
      </div>

      <!-- Resource Limits -->
      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block text-sm font-medium text-slate-400 mb-1">
            Worker threads (0 = auto)
          </label>
          <input
            type="number"
            class="input"
            min="0"
            max="64"
            bind:value={config.workers}
          />
        </div>
        <div>
          <label class="block text-sm font-medium text-slate-400 mb-1">
            Chunk size (MiB)
          </label>
          <input
            type="number"
            class="input"
            min="16"
            max="512"
            bind:value={config.chunk_size_mib}
          />
        </div>
        <div>
          <label class="block text-sm font-medium text-slate-400 mb-1">
            Overlap (KiB)
          </label>
          <input
            type="number"
            class="input"
            min="0"
            max="64"
            bind:value={config.overlap_kib}
          />
        </div>
        <div>
          <label class="block text-sm font-medium text-slate-400 mb-1">
            Metadata format
          </label>
          <select class="input" bind:value={config.metadata_backend}>
            <option value="jsonl">JSON Lines</option>
            <option value="csv">CSV</option>
            <option value="parquet">Parquet</option>
          </select>
        </div>
      </div>
    </div>
  </details>

  <!-- Start Button -->
  <div class="flex justify-end gap-4">
    <button
      class="btn btn-primary px-8 py-3 text-lg"
      disabled={!canStart || isStarting}
      onclick={handleStartScan}
    >
      {#if isStarting}
        <span class="animate-spin mr-2">⏳</span>
        Starting...
      {:else}
        🚀 Start Scan
      {/if}
    </button>
  </div>
</div>
