// src/lib/stores/ui.svelte.ts
// UI state management

import type { ViewTab, RecentScan } from '$lib/api/types';

interface Notification {
  id: string;
  type: 'info' | 'success' | 'error' | 'warning';
  title: string;
  message: string;
}

function createUiStore() {
  let currentTab = $state<ViewTab>('dashboard');
  let sidebarCollapsed = $state(false);
  let theme = $state<'dark' | 'light'>('dark');
  let recentScans = $state<RecentScan[]>([]);
  let toasts = $state<Array<{ id: string; message: string; type: 'info' | 'success' | 'error' | 'warning' }>>([]);
  let notifications = $state<Notification[]>([]);

  return {
    // Getters
    get currentTab() { return currentTab; },
    get sidebarCollapsed() { return sidebarCollapsed; },
    get theme() { return theme; },
    get recentScans() { return recentScans; },
    get toasts() { return toasts; },
    get notifications() { return notifications; },

    // Actions
    setTab(tab: ViewTab) {
      currentTab = tab;
    },

    toggleSidebar() {
      sidebarCollapsed = !sidebarCollapsed;
    },

    setTheme(newTheme: 'dark' | 'light') {
      theme = newTheme;
      if (typeof document !== 'undefined') {
        document.documentElement.classList.toggle('dark', newTheme === 'dark');
      }
    },

    addRecentScan(scan: RecentScan) {
      // Add to front, keep max 10
      recentScans = [scan, ...recentScans.filter(s => s.run_id !== scan.run_id).slice(0, 9)];
    },

    showToast(message: string, type: 'info' | 'success' | 'error' | 'warning' = 'info') {
      const id = crypto.randomUUID();
      toasts = [...toasts, { id, message, type }];
      
      // Auto-dismiss after 5 seconds
      setTimeout(() => {
        toasts = toasts.filter(t => t.id !== id);
      }, 5000);
    },

    dismissToast(id: string) {
      toasts = toasts.filter(t => t.id !== id);
    },

    addNotification(notification: Omit<Notification, 'id'>) {
      const id = crypto.randomUUID();
      notifications = [...notifications, { ...notification, id }];
      
      // Auto-dismiss after 5 seconds
      setTimeout(() => {
        notifications = notifications.filter(n => n.id !== id);
      }, 5000);
    },

    dismissNotification(id: string) {
      notifications = notifications.filter(n => n.id !== id);
    },
  };
}

export const uiStore = createUiStore();
