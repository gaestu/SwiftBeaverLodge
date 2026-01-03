<script lang="ts">
  import { scanStore, configStore } from '$lib/stores';
  import { stopScan } from '$lib/api/commands';
  import { formatBytes, formatDuration, formatThroughput, formatNumber } from '$lib/utils';

  let isStopping = $state(false);

  async function handleStop() {
    isStopping = true;
    try {
      await stopScan();
      scanStore.cancel();
    } catch (e) {
      console.error('Failed to stop scan:', e);
    } finally {
      isStopping = false;
    }
  }

  // Derived values
  let progress = $derived(scanStore.progress);
  let percentage = $derived(scanStore.percentage);
  let isRunning = $derived(scanStore.isRunning);
  let logs = $derived(scanStore.logs);
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <h2 class="text-2xl font-bold text-white">Scan Monitor</h2>
    {#if isRunning}
      <button
        class="btn btn-danger"
        disabled={isStopping}
        onclick={handleStop}
      >
        {#if isStopping}
          Stopping...
        {:else}
          ⏹️ Stop Scan
        {/if}
      </button>
    {/if}
  </div>

  {#if !isRunning && !progress}
    <div class="card text-center py-12">
      <div class="text-6xl mb-4">📊</div>
      <h3 class="text-xl font-semibold text-slate-300 mb-2">No Active Scan</h3>
      <p class="text-slate-500">Configure and start a scan to see progress here.</p>
    </div>
  {:else}
    <!-- Progress Overview -->
    <div class="card">
      <div class="flex items-center justify-between mb-4">
        <h3 class="card-header mb-0">Progress</h3>
        <div class="flex items-center gap-2">
          {#if isRunning}
            <span class="flex h-3 w-3 relative">
              <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
              <span class="relative inline-flex rounded-full h-3 w-3 bg-green-500"></span>
            </span>
            <span class="text-green-400 font-medium">Running</span>
          {:else if scanStore.status === 'completed'}
            <span class="h-3 w-3 rounded-full bg-blue-500"></span>
            <span class="text-blue-400 font-medium">Completed</span>
          {:else if scanStore.status === 'failed'}
            <span class="h-3 w-3 rounded-full bg-red-500"></span>
            <span class="text-red-400 font-medium">Failed</span>
          {:else if scanStore.status === 'cancelled'}
            <span class="h-3 w-3 rounded-full bg-yellow-500"></span>
            <span class="text-yellow-400 font-medium">Cancelled</span>
          {/if}
        </div>
      </div>

      <!-- Progress Bar -->
      <div class="mb-4">
        <div class="flex justify-between text-sm text-slate-400 mb-2">
          <span>{percentage.toFixed(1)}%</span>
          <span>
            {#if progress}
              {formatBytes(progress.bytes_scanned)} / {formatBytes(progress.total_bytes)}
            {/if}
          </span>
        </div>
        <div class="progress-bar h-4 rounded-lg">
          <div 
            class="progress-bar-fill"
            style="width: {percentage}%"
          ></div>
        </div>
      </div>

      <!-- Stats Grid -->
      {#if progress}
        <div class="grid grid-cols-4 gap-4">
          <div class="bg-slate-700/50 rounded-lg p-4 text-center">
            <div class="text-2xl font-bold text-white">{formatThroughput(progress.throughput_mib)}</div>
            <div class="text-xs text-slate-400">Throughput</div>
          </div>
          <div class="bg-slate-700/50 rounded-lg p-4 text-center">
            <div class="text-2xl font-bold text-white">{formatDuration(progress.elapsed_seconds)}</div>
            <div class="text-xs text-slate-400">Elapsed</div>
          </div>
          <div class="bg-slate-700/50 rounded-lg p-4 text-center">
            <div class="text-2xl font-bold text-white">
              {progress.eta_seconds ? formatDuration(progress.eta_seconds) : '--:--'}
            </div>
            <div class="text-xs text-slate-400">ETA</div>
          </div>
          <div class="bg-slate-700/50 rounded-lg p-4 text-center">
            <div class="text-2xl font-bold text-white">{formatNumber(progress.chunks_processed)}</div>
            <div class="text-xs text-slate-400">Chunks</div>
          </div>
        </div>
      {/if}
    </div>

    <!-- Discovery Stats -->
    {#if progress}
      <div class="grid grid-cols-3 gap-4">
        <!-- Files Carved -->
        <div class="card">
          <div class="flex items-center gap-3">
            <div class="w-12 h-12 rounded-lg bg-green-900/50 flex items-center justify-center">
              <span class="text-2xl">📁</span>
            </div>
            <div>
              <div class="text-3xl font-bold text-white">{formatNumber(progress.files_carved)}</div>
              <div class="text-sm text-slate-400">Files Carved</div>
            </div>
          </div>
        </div>

        <!-- Signature Hits -->
        <div class="card">
          <div class="flex items-center gap-3">
            <div class="w-12 h-12 rounded-lg bg-blue-900/50 flex items-center justify-center">
              <span class="text-2xl">🎯</span>
            </div>
            <div>
              <div class="text-3xl font-bold text-white">{formatNumber(progress.hits_found)}</div>
              <div class="text-sm text-slate-400">Signature Hits</div>
            </div>
          </div>
        </div>

        <!-- Artefacts -->
        <div class="card">
          <div class="flex items-center gap-3">
            <div class="w-12 h-12 rounded-lg bg-purple-900/50 flex items-center justify-center">
              <span class="text-2xl">🔗</span>
            </div>
            <div>
              <div class="text-3xl font-bold text-white">{formatNumber(progress.artefacts_extracted)}</div>
              <div class="text-sm text-slate-400">Artefacts</div>
            </div>
          </div>
        </div>
      </div>
    {/if}

    <!-- Detailed Stats -->
    {#if progress}
      <div class="card">
        <h3 class="card-header">📈 Detailed Statistics</h3>
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
          <div class="flex justify-between">
            <span class="text-slate-400">String spans:</span>
            <span class="text-white font-medium">{formatNumber(progress.string_spans)}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-slate-400">Carve errors:</span>
            <span class="{progress.carve_errors > 0 ? 'text-yellow-400' : 'text-white'} font-medium">
              {formatNumber(progress.carve_errors)}
            </span>
          </div>
          <div class="flex justify-between">
            <span class="text-slate-400">Metadata errors:</span>
            <span class="{progress.metadata_errors > 0 ? 'text-yellow-400' : 'text-white'} font-medium">
              {formatNumber(progress.metadata_errors)}
            </span>
          </div>
          <div class="flex justify-between">
            <span class="text-slate-400">SQLite errors:</span>
            <span class="{progress.sqlite_errors > 0 ? 'text-yellow-400' : 'text-white'} font-medium">
              {formatNumber(progress.sqlite_errors)}
            </span>
          </div>
        </div>
      </div>
    {/if}

    <!-- File Type Distribution -->
    {#if scanStore.fileCountsByType && Object.keys(scanStore.fileCountsByType).length > 0}
      <div class="card">
        <h3 class="card-header">📊 Files by Type</h3>
        <div class="grid grid-cols-4 md:grid-cols-6 gap-2">
          {#each Object.entries(scanStore.fileCountsByType) as [type, count]}
            <div class="bg-slate-700/50 rounded-lg px-3 py-2 text-center">
              <div class="text-lg font-bold text-white">{count}</div>
              <div class="text-xs text-slate-400 uppercase">{type}</div>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Log Output -->
    <div class="card">
      <h3 class="card-header">📝 Log Output</h3>
      <div class="bg-slate-900 rounded-lg p-4 h-48 overflow-y-auto font-mono text-xs">
        {#if logs.length === 0}
          <p class="text-slate-500">No log messages yet...</p>
        {:else}
          {#each logs as log}
            <div class="flex gap-2 mb-1">
              <span class="text-slate-500 shrink-0">
                {new Date(log.timestamp).toLocaleTimeString()}
              </span>
              <span class="{log.level === 'error' ? 'text-red-400' : 
                           log.level === 'warn' ? 'text-yellow-400' : 
                           'text-slate-300'}">
                {log.message}
              </span>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</div>
