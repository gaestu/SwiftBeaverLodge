<script lang="ts">
  import { onMount } from 'svelte';
  import { scanStore, configStore } from '$lib/stores';
  import { readCarvedMetadata, readStringArtefacts, readRunSummary, generateThumbnail } from '$lib/api/commands';
  import { formatBytes, formatNumber } from '$lib/utils';
  import type { CarvedFile, StringArtefact, RunSummary } from '$lib/api/types';

  // State
  let carvedFiles = $state<CarvedFile[]>([]);
  let stringArtefacts = $state<StringArtefact[]>([]);
  let runSummary = $state<RunSummary | null>(null);
  let isLoading = $state(false);
  let activeResultTab = $state<'files' | 'strings' | 'summary'>('files');
  let selectedFile = $state<CarvedFile | null>(null);
  let thumbnailCache = $state<Record<string, string>>({});
  
  // Filtering
  let fileTypeFilter = $state<string>('all');
  let searchQuery = $state('');

  // Load results when run_id changes
  $effect(() => {
    if (scanStore.runId) {
      loadResults();
    }
  });

  async function loadResults() {
    if (!scanStore.runId || !configStore.config.output_path) return;

    isLoading = true;
    const runPath = `${configStore.config.output_path}/${scanStore.runId}`;

    try {
      const [files, strings, summary] = await Promise.all([
        readCarvedMetadata(runPath),
        readStringArtefacts(runPath),
        readRunSummary(runPath),
      ]);

      carvedFiles = files;
      stringArtefacts = strings;
      runSummary = summary;
    } catch (e) {
      console.error('Failed to load results:', e);
    } finally {
      isLoading = false;
    }
  }

  // Derived: filtered files
  let filteredFiles = $derived(() => {
    let files = carvedFiles;
    
    if (fileTypeFilter !== 'all') {
      files = files.filter(f => f.file_type === fileTypeFilter);
    }
    
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      files = files.filter(f => 
        f.path.toLowerCase().includes(query) ||
        f.file_type.toLowerCase().includes(query)
      );
    }
    
    return files;
  });

  // Derived: unique file types
  let fileTypes = $derived([...new Set(carvedFiles.map(f => f.file_type))].sort());

  // Derived: filtered strings
  let filteredStrings = $derived(() => {
    if (!searchQuery) return stringArtefacts;
    const query = searchQuery.toLowerCase();
    return stringArtefacts.filter(s => s.content.toLowerCase().includes(query));
  });

  // Load thumbnail for a file
  async function loadThumbnail(file: CarvedFile) {
    if (thumbnailCache[file.path]) return;
    if (!['jpeg', 'png', 'gif', 'bmp', 'webp'].includes(file.file_type)) return;

    try {
      const thumbnail = await generateThumbnail(file.path, 128);
      thumbnailCache = { ...thumbnailCache, [file.path]: thumbnail };
    } catch (e) {
      console.error('Failed to load thumbnail:', e);
    }
  }

  function getFileIcon(fileType: string): string {
    const iconMap: Record<string, string> = {
      jpeg: '🖼️', png: '🖼️', gif: '🖼️', bmp: '🖼️', webp: '🖼️', tiff: '🖼️',
      pdf: '📄', docx: '📝', xlsx: '📊', pptx: '📽️',
      zip: '📦', rar: '📦', '7z': '📦',
      sqlite: '🗄️', mdb: '🗄️',
      mp3: '🎵', mp4: '🎬', avi: '🎬',
      exe: '⚙️', dll: '⚙️',
    };
    return iconMap[fileType] || '📁';
  }
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <h2 class="text-2xl font-bold text-white">Results</h2>
    {#if scanStore.runId}
      <div class="text-sm text-slate-400">
        Run: <span class="text-white font-mono">{scanStore.runId}</span>
      </div>
    {/if}
  </div>

  {#if !scanStore.runId}
    <div class="card text-center py-12">
      <div class="text-6xl mb-4">📁</div>
      <h3 class="text-xl font-semibold text-slate-300 mb-2">No Results Yet</h3>
      <p class="text-slate-500">Complete a scan to view recovered files and artefacts.</p>
    </div>
  {:else if isLoading}
    <div class="card text-center py-12">
      <div class="text-4xl animate-spin mb-4">⏳</div>
      <p class="text-slate-400">Loading results...</p>
    </div>
  {:else}
    <!-- Summary Stats -->
    {#if runSummary}
      <div class="grid grid-cols-4 gap-4">
        <div class="card text-center">
          <div class="text-3xl font-bold text-white">{formatNumber(runSummary.files_carved)}</div>
          <div class="text-sm text-slate-400">Files Recovered</div>
        </div>
        <div class="card text-center">
          <div class="text-3xl font-bold text-white">{formatNumber(runSummary.artefacts_extracted)}</div>
          <div class="text-sm text-slate-400">Artefacts Found</div>
        </div>
        <div class="card text-center">
          <div class="text-3xl font-bold text-white">{formatBytes(runSummary.bytes_scanned)}</div>
          <div class="text-sm text-slate-400">Data Scanned</div>
        </div>
        <div class="card text-center">
          <div class="text-3xl font-bold text-white">{formatNumber(runSummary.hits_found)}</div>
          <div class="text-sm text-slate-400">Signature Hits</div>
        </div>
      </div>
    {/if}

    <!-- Tab Navigation -->
    <div class="flex gap-2 border-b border-slate-700">
      <button
        class="px-4 py-2 font-medium transition-colors
               {activeResultTab === 'files' 
                 ? 'text-blue-400 border-b-2 border-blue-400' 
                 : 'text-slate-400 hover:text-slate-300'}"
        onclick={() => activeResultTab = 'files'}
      >
        📁 Files ({carvedFiles.length})
      </button>
      <button
        class="px-4 py-2 font-medium transition-colors
               {activeResultTab === 'strings' 
                 ? 'text-blue-400 border-b-2 border-blue-400' 
                 : 'text-slate-400 hover:text-slate-300'}"
        onclick={() => activeResultTab = 'strings'}
      >
        🔗 Strings ({stringArtefacts.length})
      </button>
      <button
        class="px-4 py-2 font-medium transition-colors
               {activeResultTab === 'summary' 
                 ? 'text-blue-400 border-b-2 border-blue-400' 
                 : 'text-slate-400 hover:text-slate-300'}"
        onclick={() => activeResultTab = 'summary'}
      >
        📊 Summary
      </button>
    </div>

    <!-- Search & Filter -->
    <div class="flex gap-4">
      <input
        type="text"
        class="input flex-1"
        placeholder="Search..."
        bind:value={searchQuery}
      />
      {#if activeResultTab === 'files'}
        <select class="input w-48" bind:value={fileTypeFilter}>
          <option value="all">All Types</option>
          {#each fileTypes as type}
            <option value={type}>{type.toUpperCase()}</option>
          {/each}
        </select>
      {/if}
    </div>

    <!-- Content -->
    {#if activeResultTab === 'files'}
      <div class="card">
        <div class="overflow-x-auto">
          <table class="table">
            <thead>
              <tr>
                <th>Preview</th>
                <th>Type</th>
                <th>Path</th>
                <th>Size</th>
                <th>Offset</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>
              {#each filteredFiles() as file}
                <tr 
                  class="cursor-pointer"
                  onclick={() => selectedFile = file}
                  onmouseenter={() => loadThumbnail(file)}
                >
                  <td>
                    {#if thumbnailCache[file.path]}
                      <img 
                        src={thumbnailCache[file.path]} 
                        alt="Preview" 
                        class="w-10 h-10 object-cover rounded"
                      />
                    {:else}
                      <span class="text-2xl">{getFileIcon(file.file_type)}</span>
                    {/if}
                  </td>
                  <td>
                    <span class="badge badge-info">{file.file_type.toUpperCase()}</span>
                  </td>
                  <td class="max-w-xs truncate font-mono text-xs" title={file.path}>
                    {file.path.split('/').pop()}
                  </td>
                  <td class="text-slate-400">{formatBytes(file.size)}</td>
                  <td class="font-mono text-xs text-slate-500">0x{file.global_start.toString(16)}</td>
                  <td>
                    {#if file.validated}
                      <span class="badge badge-success">✓ Valid</span>
                    {:else if file.truncated}
                      <span class="badge badge-warning">⚠ Truncated</span>
                    {:else}
                      <span class="badge">Recovered</span>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

        {#if filteredFiles().length === 0}
          <div class="text-center py-8 text-slate-500">
            No files found matching your criteria
          </div>
        {/if}
      </div>
    {:else if activeResultTab === 'strings'}
      <div class="card">
        <div class="overflow-x-auto">
          <table class="table">
            <thead>
              <tr>
                <th>Type</th>
                <th>Content</th>
                <th>Offset</th>
                <th>Encoding</th>
              </tr>
            </thead>
            <tbody>
              {#each filteredStrings() as artefact}
                <tr>
                  <td>
                    <span class="badge {artefact.artefact_kind === 'url' ? 'badge-info' :
                                        artefact.artefact_kind === 'email' ? 'badge-success' :
                                        artefact.artefact_kind === 'phone' ? 'badge-warning' :
                                        ''}">
                      {artefact.artefact_kind}
                    </span>
                  </td>
                  <td class="max-w-md truncate font-mono text-xs" title={artefact.content}>
                    {artefact.content}
                  </td>
                  <td class="font-mono text-xs text-slate-500">0x{artefact.global_start.toString(16)}</td>
                  <td class="text-slate-400">{artefact.encoding}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

        {#if filteredStrings().length === 0}
          <div class="text-center py-8 text-slate-500">
            No string artefacts found
          </div>
        {/if}
      </div>
    {:else if activeResultTab === 'summary'}
      {#if runSummary}
        <div class="card">
          <h3 class="card-header">Run Summary</h3>
          <dl class="grid grid-cols-2 gap-4">
            <div class="flex justify-between py-2 border-b border-slate-700">
              <dt class="text-slate-400">Run ID</dt>
              <dd class="font-mono">{runSummary.run_id}</dd>
            </div>
            <div class="flex justify-between py-2 border-b border-slate-700">
              <dt class="text-slate-400">Bytes Scanned</dt>
              <dd>{formatBytes(runSummary.bytes_scanned)}</dd>
            </div>
            <div class="flex justify-between py-2 border-b border-slate-700">
              <dt class="text-slate-400">Chunks Processed</dt>
              <dd>{formatNumber(runSummary.chunks_processed)}</dd>
            </div>
            <div class="flex justify-between py-2 border-b border-slate-700">
              <dt class="text-slate-400">Signature Hits</dt>
              <dd>{formatNumber(runSummary.hits_found)}</dd>
            </div>
            <div class="flex justify-between py-2 border-b border-slate-700">
              <dt class="text-slate-400">Files Carved</dt>
              <dd>{formatNumber(runSummary.files_carved)}</dd>
            </div>
            <div class="flex justify-between py-2 border-b border-slate-700">
              <dt class="text-slate-400">String Spans</dt>
              <dd>{formatNumber(runSummary.string_spans)}</dd>
            </div>
            <div class="flex justify-between py-2 border-b border-slate-700">
              <dt class="text-slate-400">Artefacts Extracted</dt>
              <dd>{formatNumber(runSummary.artefacts_extracted)}</dd>
            </div>
          </dl>
        </div>
      {:else}
        <div class="card text-center py-8 text-slate-500">
          No summary available
        </div>
      {/if}
    {/if}
  {/if}
</div>

<!-- File Detail Modal -->
{#if selectedFile}
  <div 
    class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
    onclick={() => selectedFile = null}
    onkeydown={(e) => e.key === 'Escape' && (selectedFile = null)}
    role="dialog"
    tabindex="-1"
  >
    <div 
      class="bg-slate-800 rounded-xl p-6 max-w-2xl w-full mx-4 max-h-[80vh] overflow-y-auto"
      onclick={(e) => e.stopPropagation()}
      role="document"
    >
      <div class="flex items-center justify-between mb-4">
        <h3 class="text-xl font-semibold text-white">File Details</h3>
        <button 
          class="text-slate-400 hover:text-white text-2xl"
          onclick={() => selectedFile = null}
        >
          ×
        </button>
      </div>

      {#if thumbnailCache[selectedFile.path]}
        <div class="mb-4 flex justify-center">
          <img 
            src={thumbnailCache[selectedFile.path]} 
            alt="Preview" 
            class="max-w-full max-h-64 rounded-lg"
          />
        </div>
      {/if}

      <dl class="space-y-2 text-sm">
        <div class="flex justify-between py-2 border-b border-slate-700">
          <dt class="text-slate-400">File Type</dt>
          <dd class="font-medium">{selectedFile.file_type.toUpperCase()}</dd>
        </div>
        <div class="flex justify-between py-2 border-b border-slate-700">
          <dt class="text-slate-400">Path</dt>
          <dd class="font-mono text-xs truncate max-w-sm">{selectedFile.path}</dd>
        </div>
        <div class="flex justify-between py-2 border-b border-slate-700">
          <dt class="text-slate-400">Size</dt>
          <dd>{formatBytes(selectedFile.size)}</dd>
        </div>
        <div class="flex justify-between py-2 border-b border-slate-700">
          <dt class="text-slate-400">Offset Range</dt>
          <dd class="font-mono text-xs">
            0x{selectedFile.global_start.toString(16)} - 0x{selectedFile.global_end.toString(16)}
          </dd>
        </div>
        {#if selectedFile.sha256}
          <div class="flex justify-between py-2 border-b border-slate-700">
            <dt class="text-slate-400">SHA-256</dt>
            <dd class="font-mono text-xs truncate max-w-sm">{selectedFile.sha256}</dd>
          </div>
        {/if}
        {#if selectedFile.md5}
          <div class="flex justify-between py-2 border-b border-slate-700">
            <dt class="text-slate-400">MD5</dt>
            <dd class="font-mono text-xs">{selectedFile.md5}</dd>
          </div>
        {/if}
        <div class="flex justify-between py-2 border-b border-slate-700">
          <dt class="text-slate-400">Validated</dt>
          <dd>{selectedFile.validated ? '✓ Yes' : 'No'}</dd>
        </div>
        <div class="flex justify-between py-2 border-b border-slate-700">
          <dt class="text-slate-400">Truncated</dt>
          <dd>{selectedFile.truncated ? '⚠ Yes' : 'No'}</dd>
        </div>
        {#if selectedFile.errors && selectedFile.errors.length > 0}
          <div class="py-2">
            <dt class="text-slate-400 mb-2">Errors</dt>
            <dd class="text-red-400 text-xs">
              {#each selectedFile.errors as error}
                <div>{error}</div>
              {/each}
            </dd>
          </div>
        {/if}
      </dl>
    </div>
  </div>
{/if}
