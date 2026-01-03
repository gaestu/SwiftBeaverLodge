<script lang="ts">
  import { onMount } from 'svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import ConfigPanel from '$lib/components/ConfigPanel.svelte';
  import MonitorPanel from '$lib/components/MonitorPanel.svelte';
  import ResultsPanel from '$lib/components/ResultsPanel.svelte';
  import Header from '$lib/components/Header.svelte';
  import { uiStore, scanStore } from '$lib/stores';
  import { getSystemInfo, getSwiftBeaverVersion } from '$lib/api/commands';

  // Tab state
  let activeTab = $state<'config' | 'monitor' | 'results'>('config');

  // System info
  let systemInfo = $state<any>(null);
  let version = $state<string>('');

  onMount(async () => {
    try {
      systemInfo = await getSystemInfo();
      version = await getSwiftBeaverVersion();
    } catch (e) {
      console.error('Failed to load system info:', e);
    }
  });

  // Auto-switch to monitor when scan starts
  $effect(() => {
    if (scanStore.isRunning && activeTab === 'config') {
      activeTab = 'monitor';
    }
  });

  // Auto-switch to results when scan completes
  $effect(() => {
    if (scanStore.status === 'completed' && activeTab === 'monitor') {
      activeTab = 'results';
    }
  });
</script>

<div class="flex h-screen bg-slate-900">
  <!-- Sidebar -->
  <Sidebar {activeTab} onTabChange={(tab) => activeTab = tab} />

  <!-- Main Content -->
  <div class="flex-1 flex flex-col overflow-hidden">
    <!-- Header -->
    <Header {systemInfo} {version} />

    <!-- Content Area -->
    <main class="flex-1 overflow-auto p-6">
      {#if activeTab === 'config'}
        <ConfigPanel />
      {:else if activeTab === 'monitor'}
        <MonitorPanel />
      {:else if activeTab === 'results'}
        <ResultsPanel />
      {/if}
    </main>
  </div>
</div>
