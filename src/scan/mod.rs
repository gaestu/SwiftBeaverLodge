//! Scan manager - spawns swiftbeaver binary and monitors progress

mod manager;
pub mod progress;

use std::path::{Path, PathBuf};

pub use manager::ScanManager;
pub use progress::{format_bytes, ScanProgress};

/// Minimum supported SwiftBeaver version for the schemas and flags Lodge exposes.
pub const MIN_SWIFTBEAVER_VERSION: (u32, u32, u32) = (0, 6, 7);

/// Scan state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanState {
    Idle,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Log entry from swiftbeaver
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

/// Why a discovered swiftbeaver binary is not usable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompatibilityIssue {
    /// `swiftbeaver --version` did not return parseable output.
    UnparseableVersion(String),
    /// Detected version is older than [`MIN_SWIFTBEAVER_VERSION`].
    UnsupportedVersion {
        detected: (u32, u32, u32),
        minimum: (u32, u32, u32),
    },
}

impl CompatibilityIssue {
    /// Render an actionable message suitable for the UI or log output.
    pub fn user_message(&self) -> String {
        let (mj, mn, pt) = MIN_SWIFTBEAVER_VERSION;
        match self {
            CompatibilityIssue::UnparseableVersion(raw) => format!(
                "Could not parse swiftbeaver --version output ({:?}). Requires v{}.{}.{}+.",
                raw, mj, mn, pt
            ),
            CompatibilityIssue::UnsupportedVersion {
                detected: (a, b, c),
                ..
            } => format!(
                "swiftbeaver {}.{}.{} is older than the minimum supported version v{}.{}.{}. \
                 Please upgrade SwiftBeaver.",
                a, b, c, mj, mn, pt
            ),
        }
    }
}

/// A discovered `swiftbeaver` CLI along with version metadata.
#[derive(Debug, Clone)]
pub struct DiscoveredBinary {
    pub path: PathBuf,
    /// Parsed semantic version (major, minor, patch), if `--version` succeeded
    /// and produced parseable output.
    pub version: Option<(u32, u32, u32)>,
    /// Raw `--version` output (trimmed), if available.
    pub version_string: Option<String>,
}

impl DiscoveredBinary {
    /// Returns `Ok(())` when the discovered binary meets the minimum version,
    /// or a [`CompatibilityIssue`] describing why it does not.
    pub fn check_compatibility(&self) -> Result<(), CompatibilityIssue> {
        match self.version {
            Some(v) if meets_minimum_version(v) => Ok(()),
            Some(detected) => Err(CompatibilityIssue::UnsupportedVersion {
                detected,
                minimum: MIN_SWIFTBEAVER_VERSION,
            }),
            None => Err(CompatibilityIssue::UnparseableVersion(
                self.version_string.clone().unwrap_or_default(),
            )),
        }
    }
}

/// Compare a parsed version against [`MIN_SWIFTBEAVER_VERSION`].
pub fn meets_minimum_version(version: (u32, u32, u32)) -> bool {
    version >= MIN_SWIFTBEAVER_VERSION
}

/// Parse `swiftbeaver 0.5.1`, `swiftbeaver v0.5.1`, or just `0.5.1[-suffix]`
/// into a `(major, minor, patch)` tuple.
pub fn parse_swiftbeaver_version(s: &str) -> Option<(u32, u32, u32)> {
    let trimmed = s.trim();
    // Strip optional `swiftbeaver ` prefix.
    let rest = trimmed
        .strip_prefix("swiftbeaver")
        .unwrap_or(trimmed)
        .trim();
    // Take the first whitespace-separated token, then strip a leading `v`
    // and any pre-release/build suffix.
    let token = rest.split_whitespace().next().unwrap_or("");
    let token = token.strip_prefix('v').unwrap_or(token);
    let core = token.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

/// Run `<binary> --version` and return the trimmed stdout, if successful.
/// Maximum time to wait for `swiftbeaver --version` before giving up.
///
/// Bounded so a hung or broken binary cannot freeze any caller (including the
/// UI thread, which probes at startup and on the Help → Refresh action).
const VERSION_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

/// Run `<binary> --version` and return the trimmed stdout, if successful.
///
/// Enforces [`VERSION_PROBE_TIMEOUT`]: if the child does not exit in time it
/// is killed and `None` is returned.
fn read_version_output(path: &Path) -> Option<String> {
    use std::process::Stdio;

    let mut child = std::process::Command::new(path)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let deadline = std::time::Instant::now() + VERSION_PROBE_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let mut buf = String::new();
                if let Some(mut out) = child.stdout.take() {
                    use std::io::Read;
                    let _ = out.read_to_string(&mut buf);
                }
                return Some(buf.trim().to_string());
            }
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    tracing::warn!(
                        "swiftbeaver --version timed out after {:?}; treating as unparseable",
                        VERSION_PROBE_TIMEOUT
                    );
                    return None;
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            Err(_) => return None,
        }
    }
}

/// Return candidate file names for a logical executable name on a platform.
fn binary_name_candidates(name: &str, windows: bool) -> Vec<String> {
    if windows && !name.to_ascii_lowercase().ends_with(".exe") {
        vec![format!("{name}.exe"), name.to_string()]
    } else {
        vec![name.to_string()]
    }
}

/// Pure helper: return the first platform-specific `<dir>/<name>` that exists on disk.
///
/// Factored out so tests can exercise the search order without mutating
/// process-global state (PATH, current_exe, cwd).
fn find_in_dirs(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    find_in_dirs_for_platform(name, dirs, cfg!(windows))
}

fn find_in_dirs_for_platform(name: &str, dirs: &[PathBuf], windows: bool) -> Option<PathBuf> {
    let names = binary_name_candidates(name, windows);
    for dir in dirs {
        for candidate_name in &names {
            let candidate = dir.join(candidate_name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Search the standard locations for a binary with the given name.
///
/// Order:
/// 1. `<exe_dir>/bin/<name>`
/// 2. `./bin/<name>` (current working directory)
/// 3. PATH lookup via `which`
fn find_named_binary(name: &str) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            dirs.push(exe_dir.join("bin"));
        }
    }
    dirs.push(PathBuf::from("bin"));
    if let Some(p) = find_in_dirs(name, &dirs) {
        return Some(p);
    }
    which::which(name).ok()
}

/// Find the unified `swiftbeaver` CLI.
pub fn find_swiftbeaver_binary() -> Option<PathBuf> {
    find_named_binary("swiftbeaver")
}

/// Discover the installed `swiftbeaver` CLI and read its version.
///
/// Returns `None` only when no `swiftbeaver` binary is found at all. Returned
/// binaries may still be incompatible — callers should consult
/// [`DiscoveredBinary::check_compatibility`].
pub fn discover_swiftbeaver() -> Option<DiscoveredBinary> {
    let path = find_swiftbeaver_binary()?;
    let version_string = read_version_output(&path);
    let version = version_string
        .as_deref()
        .and_then(parse_swiftbeaver_version);
    Some(DiscoveredBinary {
        path,
        version,
        version_string,
    })
}

/// Convenience: human-readable swiftbeaver version string for status displays,
/// or `"not installed"` when no binary is found.
#[allow(dead_code)]
pub fn get_swiftbeaver_version() -> String {
    discover_swiftbeaver()
        .and_then(|b| {
            b.version_string
                .or_else(|| b.version.map(|(a, b, c)| format!("{}.{}.{}", a, b, c)))
        })
        .map(|s| s.strip_prefix("swiftbeaver ").unwrap_or(&s).to_string())
        .unwrap_or_else(|| "not installed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_state_equality() {
        assert_eq!(ScanState::Idle, ScanState::Idle);
        assert_ne!(ScanState::Idle, ScanState::Running);
    }

    #[test]
    fn test_find_swiftbeaver_in_path() {
        let _ = find_swiftbeaver_binary();
    }

    #[test]
    fn test_parse_swiftbeaver_version_with_prefix() {
        assert_eq!(
            parse_swiftbeaver_version("swiftbeaver 0.5.1"),
            Some((0, 5, 1))
        );
        assert_eq!(
            parse_swiftbeaver_version("swiftbeaver 1.2.3\n"),
            Some((1, 2, 3))
        );
    }

    #[test]
    fn test_parse_swiftbeaver_version_v_prefix() {
        // `swiftbeaver vX.Y.Z` and bare `vX.Y.Z` are both common.
        assert_eq!(
            parse_swiftbeaver_version("swiftbeaver v0.5.1"),
            Some((0, 5, 1))
        );
        assert_eq!(
            parse_swiftbeaver_version("swiftbeaver v0.4.0"),
            Some((0, 4, 0))
        );
        assert_eq!(parse_swiftbeaver_version("v1.2.3"), Some((1, 2, 3)));
    }

    #[test]
    fn test_parse_swiftbeaver_version_bare() {
        assert_eq!(parse_swiftbeaver_version("0.5.1"), Some((0, 5, 1)));
        assert_eq!(
            parse_swiftbeaver_version("  10.20.30  "),
            Some((10, 20, 30))
        );
    }

    #[test]
    fn test_parse_swiftbeaver_version_two_part() {
        assert_eq!(
            parse_swiftbeaver_version("swiftbeaver 0.5"),
            Some((0, 5, 0))
        );
    }

    #[test]
    fn test_parse_swiftbeaver_version_with_suffix() {
        assert_eq!(
            parse_swiftbeaver_version("swiftbeaver 0.5.1-rc.1"),
            Some((0, 5, 1))
        );
        assert_eq!(
            parse_swiftbeaver_version("swiftbeaver 0.5.1+build.7"),
            Some((0, 5, 1))
        );
        assert_eq!(
            parse_swiftbeaver_version("swiftbeaver v0.5.1-rc.1"),
            Some((0, 5, 1))
        );
    }

    #[test]
    fn test_parse_swiftbeaver_version_invalid() {
        assert_eq!(parse_swiftbeaver_version(""), None);
        assert_eq!(parse_swiftbeaver_version("swiftbeaver"), None);
        assert_eq!(parse_swiftbeaver_version("not a version"), None);
        assert_eq!(parse_swiftbeaver_version("swiftbeaver vX.Y.Z"), None);
    }

    #[test]
    fn test_meets_minimum_version() {
        assert!(meets_minimum_version((0, 6, 7)));
        assert!(meets_minimum_version((0, 6, 8)));
        assert!(meets_minimum_version((1, 0, 0)));
        assert!(!meets_minimum_version((0, 6, 6)));
        assert!(!meets_minimum_version((0, 5, 99)));
    }

    #[test]
    fn test_compatibility_old_version_blocks() {
        let b = DiscoveredBinary {
            path: PathBuf::from("/tmp/swiftbeaver"),
            version: Some((0, 6, 6)),
            version_string: Some("swiftbeaver 0.6.6".to_string()),
        };
        assert!(b.check_compatibility().is_err());
        assert_eq!(
            b.check_compatibility(),
            Err(CompatibilityIssue::UnsupportedVersion {
                detected: (0, 6, 6),
                minimum: MIN_SWIFTBEAVER_VERSION,
            })
        );
    }

    #[test]
    fn test_compatibility_unparseable_version_blocks() {
        let b = DiscoveredBinary {
            path: PathBuf::from("/tmp/swiftbeaver"),
            version: None,
            version_string: Some("garbage output".to_string()),
        };
        assert!(b.check_compatibility().is_err());
        assert!(matches!(
            b.check_compatibility(),
            Err(CompatibilityIssue::UnparseableVersion(_))
        ));
    }

    #[test]
    fn test_compatibility_minimum_version_passes() {
        let b = DiscoveredBinary {
            path: PathBuf::from("/tmp/swiftbeaver"),
            version: Some(MIN_SWIFTBEAVER_VERSION),
            version_string: Some("swiftbeaver 0.6.7".to_string()),
        };
        assert!(b.check_compatibility().is_ok());
    }

    #[test]
    fn test_find_in_dirs_returns_first_match() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir1 = tmp.path().join("a");
        let dir2 = tmp.path().join("b");
        std::fs::create_dir_all(&dir1).unwrap();
        std::fs::create_dir_all(&dir2).unwrap();
        let target = dir2.join("swiftbeaver");
        std::fs::write(&target, b"#!/bin/sh\n").unwrap();
        assert_eq!(
            find_in_dirs("swiftbeaver", &[dir1.clone(), dir2.clone()]),
            Some(target.clone())
        );
        // dir1 takes precedence when both contain the file.
        let target1 = dir1.join("swiftbeaver");
        std::fs::write(&target1, b"#!/bin/sh\n").unwrap();
        assert_eq!(find_in_dirs("swiftbeaver", &[dir1, dir2]), Some(target1));
    }

    #[test]
    fn test_binary_name_candidates_prefer_windows_exe() {
        assert_eq!(
            binary_name_candidates("swiftbeaver", true),
            vec!["swiftbeaver.exe".to_string(), "swiftbeaver".to_string()]
        );
        assert_eq!(
            binary_name_candidates("swiftbeaver.exe", true),
            vec!["swiftbeaver.exe".to_string()]
        );
        assert_eq!(
            binary_name_candidates("swiftbeaver", false),
            vec!["swiftbeaver".to_string()]
        );
    }

    #[test]
    fn test_find_in_dirs_supports_windows_exe_name() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dir = tmp.path().join("bin");
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("swiftbeaver.exe");
        std::fs::write(&target, b"fake exe").unwrap();

        assert_eq!(
            find_in_dirs_for_platform("swiftbeaver", &[dir], true),
            Some(target)
        );
    }

    #[test]
    fn test_find_in_dirs_returns_none_when_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert_eq!(
            find_in_dirs("swiftbeaver", &[tmp.path().to_path_buf()]),
            None
        );
    }
}
