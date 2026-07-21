//! The facet model and the per-OS collector interface.
//!
//! A *facet* is one named slice of network-relevant machine state — the firewall
//! ruleset, the routing table, DNS configuration, a set of `sysctl` values,
//! installed services, and files under directories NetworkHub owns. A *snapshot*
//! is the set of all facets at one instant. Diffing two snapshots is how the
//! uninstall contract (ARCHITECTURE.md §13.5, ADR-0014) becomes a test rather
//! than a hope: snapshot, install, use, uninstall, snapshot, and assert the diff
//! is empty.

use std::collections::BTreeMap;
use std::process::Command;

use serde::{Deserialize, Serialize};

/// One named slice of machine state, stored as normalized, sorted lines so that
/// two snapshots of an unchanged system compare equal regardless of the order
/// the OS happened to print things in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Facet {
    /// How this facet was gathered, for the reader of a diff (e.g. the command).
    pub(crate) source: String,
    /// Sorted, normalized lines. Empty is meaningful: "no rules" is a state.
    pub(crate) lines: Vec<String>,
    /// Set when collection could not run (e.g. needs root). Kept in the snapshot
    /// so a diff never silently treats "could not read" as "nothing there".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
}

impl Facet {
    /// Builds a facet from sorted, de-duplicated lines.
    pub(crate) fn from_lines(source: impl Into<String>, mut lines: Vec<String>) -> Self {
        lines.sort();
        lines.dedup();
        Self {
            source: source.into(),
            lines,
            error: None,
        }
    }

    pub(crate) fn unavailable(source: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            lines: Vec::new(),
            error: Some(error.into()),
        }
    }
}

/// A full snapshot: an ordered map of facet name to facet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Snapshot {
    pub(crate) os: String,
    pub(crate) facets: BTreeMap<String, Facet>,
}

impl Snapshot {
    pub(crate) fn new(os: impl Into<String>) -> Self {
        Self {
            os: os.into(),
            facets: BTreeMap::new(),
        }
    }

    pub(crate) fn insert(&mut self, name: impl Into<String>, facet: Facet) {
        self.facets.insert(name.into(), facet);
    }
}

/// Per-OS collectors implement this. Each `collect` gathers every facet the
/// platform can report. Collectors run **read-only** commands only.
pub(crate) trait StateCollector {
    fn os_name(&self) -> &'static str;
    fn collect(&self, config: &Config) -> Snapshot;
}

/// What to watch and what to ignore.
#[derive(Debug, Clone)]
pub(crate) struct Config {
    /// Directories whose file listings form the `files` facet. Defaults to the
    /// locations NetworkHub installs into; overridable for testing.
    pub(crate) watch_dirs: Vec<String>,
    /// Substrings that mark a line as volatile (timestamps, random ports,
    /// counters). Matching lines are dropped before comparison, so ephemeral
    /// noise does not masquerade as a real change.
    pub(crate) ignore_substrings: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            watch_dirs: default_watch_dirs(),
            ignore_substrings: Vec::new(),
        }
    }
}

#[cfg(target_os = "linux")]
fn default_watch_dirs() -> Vec<String> {
    vec![
        "/etc/systemd/system".into(),
        "/usr/local/libexec".into(),
        "/run".into(),
        "/etc/networkhub".into(),
    ]
}

#[cfg(target_os = "macos")]
fn default_watch_dirs() -> Vec<String> {
    vec![
        "/Library/LaunchDaemons".into(),
        "/usr/local/libexec".into(),
        "/var/run".into(),
        "/Library/Application Support/NetworkHub".into(),
    ]
}

#[cfg(target_os = "windows")]
fn default_watch_dirs() -> Vec<String> {
    vec![
        r"C:\Program Files\NetworkHub".into(),
        r"C:\ProgramData\NetworkHub".into(),
    ]
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn default_watch_dirs() -> Vec<String> {
    Vec::new()
}

/// Runs a read-only command and returns its stdout lines, or a facet-level error
/// if it could not run. Uses an argv array — never a shell string — so that even
/// this test tool models the daemon's no-shell-interpolation rule.
pub(crate) fn run_lines(bin: &str, args: &[&str]) -> Result<Vec<String>, String> {
    let output = Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| format!("could not run {bin}: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{bin} exited {}: {}", output.status, stderr.trim()));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

/// Builds a facet by running a read-only command, recording a facet-level error
/// if it could not run.
pub(crate) fn command_facet(source: &str, bin: &str, args: &[&str]) -> Facet {
    match run_lines(bin, args) {
        Ok(lines) => Facet::from_lines(source, lines),
        Err(e) => Facet::unavailable(source, e),
    }
}

/// Builds the `files` facet from the configured watch directories.
pub(crate) fn files_facet(config: &Config) -> Facet {
    let mut lines = Vec::new();
    for dir in &config.watch_dirs {
        for f in list_dir(dir) {
            lines.push(format!("{dir}/{f}"));
        }
    }
    Facet::from_lines("watched directories", lines)
}

/// Drops lines matching any ignore substring, so volatile entries (timestamps,
/// ephemeral ports, counters) do not read as real changes in a diff.
pub(crate) fn apply_ignores(snap: &mut Snapshot, config: &Config) {
    if config.ignore_substrings.is_empty() {
        return;
    }
    for facet in snap.facets.values_mut() {
        facet
            .lines
            .retain(|line| !config.ignore_substrings.iter().any(|s| line.contains(s)));
    }
}

/// Lists files under a directory recursively, as relative paths, for the `files`
/// facet. Missing directories yield an empty list, not an error: their absence
/// is the normal pre-install state.
pub(crate) fn list_dir(path: &str) -> Vec<String> {
    fn walk(base: &std::path::Path, dir: &std::path::Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk(base, &p, out);
            } else if let Ok(rel) = p.strip_prefix(base) {
                out.push(rel.display().to_string());
            }
        }
    }
    let base = std::path::Path::new(path);
    let mut out = Vec::new();
    walk(base, base, &mut out);
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn facet_lines_are_sorted_and_deduped() {
        // Two snapshots of the same state must compare equal regardless of the
        // order the OS printed things, which is the whole basis of the diff.
        let f = Facet::from_lines("t", vec!["b".into(), "a".into(), "b".into()]);
        assert_eq!(f.lines, vec!["a".to_owned(), "b".to_owned()]);
    }

    #[test]
    fn ignore_substrings_drop_volatile_lines() {
        let mut snap = Snapshot::new("test");
        snap.insert(
            "f",
            Facet::from_lines("t", vec!["keep me".into(), "port 54123 ephemeral".into()]),
        );
        let config = Config {
            watch_dirs: vec![],
            ignore_substrings: vec!["ephemeral".into()],
        };
        apply_ignores(&mut snap, &config);
        assert_eq!(snap.facets["f"].lines, vec!["keep me".to_owned()]);
    }

    #[test]
    fn list_dir_of_missing_path_is_empty_not_error() {
        // A watched directory that does not exist is the normal pre-install
        // state, so it must be an empty listing, never a failure.
        assert!(list_dir("/definitely/not/a/real/path/networkhub").is_empty());
    }

    #[test]
    fn run_lines_reports_missing_binary_as_error() {
        let result = run_lines("this-binary-does-not-exist-xyz", &[]);
        assert!(result.is_err());
    }
}
