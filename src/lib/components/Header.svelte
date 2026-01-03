<script lang="ts">
  import type { SystemInfo } from '$lib/api/types';
  import { formatBytes } from '$lib/utils';

  interface Props {
    systemInfo: SystemInfo | null;
    version: string;
  }
  let { systemInfo, version }: Props = $props();
</script>

<header class="h-14 bg-slate-800 border-b border-slate-700 flex items-center justify-between px-6">
  <div class="flex items-center gap-4">
    <h2 class="text-lg font-semibold text-white">Forensic File Recovery</h2>
  </div>

  <div class="flex items-center gap-6 text-sm text-slate-400">
    {#if systemInfo}
      <div class="flex items-center gap-2">
        <span>💻</span>
        <span>{systemInfo.os_name ?? 'Unknown OS'}</span>
      </div>
      <div class="flex items-center gap-2">
        <span>🔧</span>
        <span>{systemInfo.cpu_cores} cores</span>
      </div>
      <div class="flex items-center gap-2">
        <span>💾</span>
        <span>{formatBytes(systemInfo.available_memory)} / {formatBytes(systemInfo.total_memory)}</span>
      </div>
      {#if systemInfo.gpus && systemInfo.gpus.length > 0}
        <div class="flex items-center gap-2">
          <span>🎮</span>
          <span>{systemInfo.gpus.length} GPU{systemInfo.gpus.length > 1 ? 's' : ''}</span>
        </div>
      {/if}
    {/if}
    {#if version}
      <div class="flex items-center gap-2 text-slate-500">
        <span>v{version}</span>
      </div>
    {/if}
  </div>
</header>
