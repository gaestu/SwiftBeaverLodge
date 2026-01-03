<script lang="ts">
  import { scanStore } from '$lib/stores';

  interface Props {
    activeTab: 'config' | 'monitor' | 'results';
    onTabChange: (tab: 'config' | 'monitor' | 'results') => void;
  }
  let { activeTab, onTabChange }: Props = $props();

  const tabs = [
    { id: 'config' as const, label: 'Configuration', icon: '⚙️' },
    { id: 'monitor' as const, label: 'Monitor', icon: '📊' },
    { id: 'results' as const, label: 'Results', icon: '📁' },
  ];
</script>

<aside class="w-64 bg-slate-900 border-r border-slate-700 flex flex-col">
  <!-- Logo -->
  <div class="p-4 border-b border-slate-700">
    <h1 class="text-xl font-bold text-white flex items-center gap-2">
      <span class="text-2xl">🦫</span>
      SwiftBeaverLodge
    </h1>
    <p class="text-xs text-slate-400 mt-1">Forensic File Carver</p>
  </div>

  <!-- Navigation -->
  <nav class="flex-1 p-4">
    <ul class="space-y-2">
      {#each tabs as tab}
        <li>
          <button
            class="w-full flex items-center gap-3 px-4 py-3 rounded-lg transition-all duration-200
                   {activeTab === tab.id 
                     ? 'bg-blue-600 text-white' 
                     : 'text-slate-300 hover:bg-slate-800'}"
            onclick={() => onTabChange(tab.id)}
          >
            <span class="text-lg">{tab.icon}</span>
            <span class="font-medium">{tab.label}</span>
            {#if tab.id === 'monitor' && scanStore.isRunning}
              <span class="ml-auto">
                <span class="flex h-2 w-2 relative">
                  <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
                  <span class="relative inline-flex rounded-full h-2 w-2 bg-green-500"></span>
                </span>
              </span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  </nav>

  <!-- Status indicator -->
  <div class="p-4 border-t border-slate-700">
    <div class="flex items-center gap-2 text-sm">
      {#if scanStore.isRunning}
        <span class="flex h-2 w-2">
          <span class="animate-ping absolute inline-flex h-2 w-2 rounded-full bg-green-400 opacity-75"></span>
          <span class="relative inline-flex rounded-full h-2 w-2 bg-green-500"></span>
        </span>
        <span class="text-green-400">Scan in progress</span>
      {:else if scanStore.status === 'completed'}
        <span class="h-2 w-2 rounded-full bg-blue-500"></span>
        <span class="text-blue-400">Scan completed</span>
      {:else if scanStore.status === 'failed'}
        <span class="h-2 w-2 rounded-full bg-red-500"></span>
        <span class="text-red-400">Scan failed</span>
      {:else}
        <span class="h-2 w-2 rounded-full bg-slate-500"></span>
        <span class="text-slate-400">Ready</span>
      {/if}
    </div>
  </div>
</aside>
