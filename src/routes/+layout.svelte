<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import { scanStore, configStore, uiStore } from '$lib/stores';
  import { listenToScanEvents, listenToStateEvents, listenToLogEvents } from '$lib/api/events';

  interface Props {
    children?: import('svelte').Snippet;
  }
  let { children }: Props = $props();

  // Subscribe to Tauri events on mount
  onMount(() => {
    const unsubscribers: (() => void)[] = [];

    // Set up event listeners
    listenToScanEvents({
      onProgress: (progress) => {
        scanStore.updateProgress(progress);
      },
      onCompleted: (data) => {
        scanStore.complete();
        uiStore.addNotification({
          type: 'success',
          title: 'Scan Completed',
          message: `Scan ${data.run_id} completed successfully`,
        });
      },
      onFailed: (error) => {
        scanStore.fail(error);
        uiStore.addNotification({
          type: 'error',
          title: 'Scan Failed',
          message: error,
        });
      },
    }).then((unsub) => unsubscribers.push(unsub));

    listenToStateEvents((state) => {
      // State is already handled by other events
    }).then((unsub) => unsubscribers.push(unsub));

    listenToLogEvents((log) => {
      scanStore.addLog(log);
    }).then((unsub) => unsubscribers.push(unsub));

    return () => {
      unsubscribers.forEach((unsub) => unsub());
    };
  });
</script>

<div class="app-container">
  {@render children?.()}
</div>

<style>
  .app-container {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }
</style>
