// src/lib/api/commands.ts
// Tauri command wrappers with TypeScript types

import { invoke } from '@tauri-apps/api/core';
import type {
  ScanConfig,
  ScanHandle,
  ScanStatus,
  SystemInfo,
  GpuInfo,
  CarvedFile,
  StringArtefact,
  BrowserHistoryRecord,
  RunSummary,
} from './types';

// =============================================================================
// Scan Commands
// =============================================================================

/**
 * Start a new carving scan
 */
export async function startScan(config: ScanConfig): Promise<ScanHandle> {
  return invoke<ScanHandle>('start_scan', { config });
}

/**
 * Stop the current scan
 */
export async function stopScan(): Promise<void> {
  return invoke<void>('stop_scan');
}

/**
 * Get current scan status
 */
export async function getScanStatus(): Promise<ScanStatus> {
  return invoke<ScanStatus>('get_scan_status');
}

// =============================================================================
// Config Commands
// =============================================================================

/**
 * Load configuration from YAML file
 */
export async function loadConfig(path: string): Promise<ScanConfig> {
  return invoke<ScanConfig>('load_config', { path });
}

/**
 * Save configuration to YAML file
 */
export async function saveConfig(config: ScanConfig, path: string): Promise<void> {
  return invoke<void>('save_config', { config, path });
}

/**
 * Get default configuration
 */
export async function getDefaultConfig(): Promise<ScanConfig> {
  return invoke<ScanConfig>('get_default_config');
}

// =============================================================================
// File Commands
// =============================================================================

/**
 * Browse for input file (opens native dialog)
 */
export async function browseInputFile(): Promise<string | null> {
  return invoke<string | null>('browse_input_file');
}

/**
 * Browse for output directory
 */
export async function browseOutputDir(): Promise<string | null> {
  return invoke<string | null>('browse_output_dir');
}

/**
 * List available block devices (Linux only)
 */
export interface BlockDevice {
  path: string;
  name: string;
  size_bytes: number;
  model?: string;
}

export async function listBlockDevices(): Promise<BlockDevice[]> {
  return invoke<BlockDevice[]>('list_block_devices');
}

/**
 * Read carved files metadata from a run
 */
export async function readCarvedMetadata(runPath: string): Promise<CarvedFile[]> {
  return invoke<CarvedFile[]>('read_carved_metadata', { runPath });
}

/**
 * Read string artefacts from a run
 */
export async function readStringArtefacts(runPath: string): Promise<StringArtefact[]> {
  return invoke<StringArtefact[]>('read_string_artefacts', { runPath });
}

/**
 * Read browser history from a run
 */
export async function readBrowserHistory(runPath: string): Promise<BrowserHistoryRecord[]> {
  return invoke<BrowserHistoryRecord[]>('read_browser_history', { runPath });
}

/**
 * Read run summary
 */
export async function readRunSummary(runPath: string): Promise<RunSummary | null> {
  return invoke<RunSummary | null>('read_run_summary', { runPath });
}

/**
 * Generate thumbnail for image file (returns base64)
 */
export async function generateThumbnail(path: string, maxSize: number = 256): Promise<string> {
  return invoke<string>('generate_thumbnail', { path, maxSize });
}

/**
 * Read file bytes for hex viewer
 */
export async function readFileBytes(path: string, offset: number, length: number): Promise<number[]> {
  return invoke<number[]>('read_file_bytes', { path, offset, length });
}

// =============================================================================
// System Commands
// =============================================================================

/**
 * Get system information
 */
export async function getSystemInfo(): Promise<SystemInfo> {
  return invoke<SystemInfo>('get_system_info');
}

/**
 * Check GPU availability
 */
export async function checkGpuSupport(): Promise<GpuInfo> {
  return invoke<GpuInfo>('check_gpu_support');
}

/**
 * Get SwiftBeaver version
 */
export async function getSwiftBeaverVersion(): Promise<string> {
  return invoke<string>('get_swiftbeaver_version');
}

/**
 * Get app version
 */
export async function getAppVersion(): Promise<string> {
  return invoke<string>('get_app_version');
}
