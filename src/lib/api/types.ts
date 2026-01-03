// src/lib/api/types.ts
// TypeScript interfaces for SwiftBeaverLodge

// =============================================================================
// Scan Configuration
// =============================================================================

export interface ScanConfig {
  // Input/Output
  input_path: string;
  output_path: string;
  compute_evidence_hash: boolean;
  evidence_sha256?: string;

  // File types to carve
  file_types: string[];
  disable_zip: boolean;

  // String scanning options
  scan_strings: boolean;
  scan_urls: boolean;
  scan_emails: boolean;
  scan_phones: boolean;
  scan_utf16: boolean;
  string_min_len: number;
  string_max_len: number;

  // GPU acceleration
  gpu_enabled: boolean;
  gpu_backend?: 'opencl' | 'cuda';

  // Entropy detection
  scan_entropy: boolean;
  entropy_window_bytes: number;
  entropy_threshold: number;

  // SQLite recovery
  scan_sqlite_pages: boolean;

  // Resource limits
  max_bytes?: number;
  max_chunks?: number;
  max_files?: number;
  max_memory_mib?: number;

  // Output settings
  overlap_kib: number;
  metadata_backend: 'jsonl' | 'csv' | 'parquet';
  workers: number;
  chunk_size_mib: number;
}

export const DEFAULT_SCAN_CONFIG: ScanConfig = {
  input_path: '',
  output_path: '',
  compute_evidence_hash: true,
  file_types: ['jpeg', 'png', 'gif', 'pdf', 'zip', 'sqlite', 'docx', 'xlsx', 'pptx'],
  disable_zip: false,
  scan_strings: true,
  scan_urls: true,
  scan_emails: true,
  scan_phones: true,
  scan_utf16: false,
  string_min_len: 8,
  string_max_len: 4096,
  gpu_enabled: false,
  scan_entropy: false,
  entropy_window_bytes: 256,
  entropy_threshold: 7.5,
  scan_sqlite_pages: false,
  overlap_kib: 4,
  metadata_backend: 'parquet', // Default to Parquet for better performance
  workers: 0, // 0 = auto-detect
  chunk_size_mib: 64,
};

// =============================================================================
// Scan Status & Progress
// =============================================================================

export type ScanState = 'idle' | 'running' | 'paused' | 'completed' | 'failed' | 'cancelled';

export interface ScanProgress {
  bytes_scanned: number;
  total_bytes: number;
  chunks_processed: number;
  hits_found: number;
  files_carved: number;
  string_spans: number;
  artefacts_extracted: number;
  carve_errors: number;
  metadata_errors: number;
  sqlite_errors: number;
  elapsed_seconds: number;
  throughput_mib: number;
  eta_seconds: number | null;
}

export interface ScanStatus {
  state: ScanState;
  run_id?: string;
  started_at?: string;
  progress?: ScanProgress;
  error?: string;
}

export interface ScanHandle {
  run_id: string;
}

// =============================================================================
// Carved Files & Artefacts
// =============================================================================

export interface CarvedFile {
  run_id: string;
  file_type: string;
  path: string;
  extension: string;
  global_start: number;
  global_end: number;
  size: number;
  md5?: string;
  sha256?: string;
  validated: boolean;
  truncated: boolean;
  errors: string[];
  pattern_id?: string;
}

export interface StringArtefact {
  run_id: string;
  artefact_kind: 'url' | 'email' | 'phone' | 'string';
  content: string;
  encoding: string;
  global_start: number;
  global_end: number;
}

export interface BrowserHistoryRecord {
  run_id: string;
  source_file: string;
  browser: string;
  url: string;
  title?: string;
  visit_count?: number;
  last_visit_time?: string;
}

export interface BrowserCookieRecord {
  run_id: string;
  source_file: string;
  browser: string;
  host: string;
  name: string;
  value?: string;
  path?: string;
  expires?: string;
  secure: boolean;
  http_only: boolean;
}

export interface RunSummary {
  run_id: string;
  bytes_scanned: number;
  chunks_processed: number;
  hits_found: number;
  files_carved: number;
  string_spans: number;
  artefacts_extracted: number;
}

export interface EntropyRegion {
  run_id: string;
  global_start: number;
  global_end: number;
  entropy: number;
  window_size: number;
}

// =============================================================================
// System Information
// =============================================================================

export interface SystemInfo {
  os_name?: string;
  os_version?: string;
  cpu_cores: number;
  cpu_model?: string;
  total_memory: number;
  available_memory: number;
  gpus: GpuInfo[];
}

export interface GpuInfo {
  name: string;
  vendor?: string;
  driver_version?: string;
  memory_bytes?: number;
  backend: string;
}

// =============================================================================
// Events (Backend → Frontend)
// =============================================================================

export interface FileCarvedEvent {
  file_type: string;
  path: string;
  offset: number;
  size: number;
  sha256?: string;
}

export interface LogMessageEvent {
  level: 'info' | 'warn' | 'error' | 'debug';
  timestamp: string;
  message: string;
}

export interface ScanStateChangedEvent {
  state: ScanState;
  message?: string;
}

export interface ArtefactFoundEvent {
  artefact_type: 'url' | 'email' | 'phone';
  value: string;
  offset: number;
}

// =============================================================================
// File Types
// =============================================================================

export const SUPPORTED_FILE_TYPES = [
  { id: 'jpeg', label: 'JPEG', category: 'images' },
  { id: 'png', label: 'PNG', category: 'images' },
  { id: 'gif', label: 'GIF', category: 'images' },
  { id: 'bmp', label: 'BMP', category: 'images' },
  { id: 'tiff', label: 'TIFF', category: 'images' },
  { id: 'webp', label: 'WEBP', category: 'images' },
  { id: 'pdf', label: 'PDF', category: 'documents' },
  { id: 'zip', label: 'ZIP', category: 'archives' },
  { id: 'docx', label: 'DOCX', category: 'documents' },
  { id: 'xlsx', label: 'XLSX', category: 'documents' },
  { id: 'pptx', label: 'PPTX', category: 'documents' },
  { id: 'rar', label: 'RAR', category: 'archives' },
  { id: '7z', label: '7z', category: 'archives' },
  { id: 'sqlite', label: 'SQLite', category: 'databases' },
  { id: 'mp4', label: 'MP4', category: 'video' },
] as const;

export type FileTypeId = typeof SUPPORTED_FILE_TYPES[number]['id'];

// =============================================================================
// UI State
// =============================================================================

export type ViewTab = 'dashboard' | 'configure' | 'monitor' | 'results' | 'reports';

export interface RecentScan {
  path: string;
  run_id: string;
  timestamp: string;
  files_carved: number;
}
