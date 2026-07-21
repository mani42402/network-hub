//! `state-diff` — snapshot and diff the network-relevant state of a machine.
//!
//! This is the tool that turns the uninstall contract (ARCHITECTURE.md §13.5,
//! ADR-0014) and the fail-open invariant (ADR-0005) into automated tests. The
//! workflow, run per-OS in CI and by hand on VM snapshots:
//!
//!     state-diff snapshot before.json
//!     <install, use, uninstall NetworkHub>
//!     state-diff snapshot after.json
//!     state-diff diff before.json after.json    # must be empty
//!
//! A non-empty diff means NetworkHub changed the machine and did not change it
//! back — a leftover firewall rule, a service, a sysctl, a file. Exit code 1 on
//! a non-empty diff makes that a failing test.

mod collect;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use std::collections::BTreeSet;
use std::process::ExitCode;

use collect::{Config, Snapshot, StateCollector};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("snapshot") => cmd_snapshot(&args[2..]),
        Some("diff") => cmd_diff(&args[2..]),
        Some("facets") => cmd_facets(),
        _ => {
            usage();
            ExitCode::FAILURE
        }
    }
}

fn usage() {
    eprintln!(
        "state-diff — snapshot and diff network-relevant machine state\n\n\
         usage:\n  \
         state-diff snapshot <out.json> [--watch DIR ...] [--ignore SUBSTR ...]\n  \
         state-diff diff <before.json> <after.json> [--ignore SUBSTR ...]\n  \
         state-diff facets\n\n\
         exit codes for `diff`: 0 = identical, 1 = differences found, 2 = error"
    );
}

fn collector() -> Box<dyn StateCollector> {
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxCollector)
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacosCollector)
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsCollector)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        compile_error!("state-diff supports Linux, macOS, and Windows only")
    }
}

fn parse_config(args: &[String]) -> Config {
    let mut config = Config::default();
    let mut extra_watch = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--watch" if i + 1 < args.len() => {
                extra_watch.push(args[i + 1].clone());
                i += 2;
            }
            "--ignore" if i + 1 < args.len() => {
                config.ignore_substrings.push(args[i + 1].clone());
                i += 2;
            }
            "--only-watch" if i + 1 < args.len() => {
                // Replace defaults entirely — used by the self-test to point the
                // files facet at a temporary directory.
                config.watch_dirs = vec![args[i + 1].clone()];
                i += 2;
            }
            _ => i += 1,
        }
    }
    config.watch_dirs.extend(extra_watch);
    config
}

fn cmd_snapshot(args: &[String]) -> ExitCode {
    let Some(out) = args.first() else {
        eprintln!("snapshot: missing output path");
        return ExitCode::FAILURE;
    };
    let config = parse_config(&args[1..]);
    let c = collector();
    let snap = c.collect(&config);
    debug_assert_eq!(snap.os, c.os_name());

    let json = match serde_json::to_string_pretty(&snap) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("serialize failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = std::fs::write(out, json) {
        eprintln!("write {out} failed: {e}");
        return ExitCode::FAILURE;
    }

    let unavailable = snap.facets.values().filter(|f| f.error.is_some()).count();
    eprintln!(
        "wrote {out}: {} facets ({unavailable} unavailable — likely need root/elevation)",
        snap.facets.len()
    );
    ExitCode::SUCCESS
}

fn cmd_diff(args: &[String]) -> ExitCode {
    let (Some(before_path), Some(after_path)) = (args.first(), args.get(1)) else {
        eprintln!("diff: need <before.json> <after.json>");
        return ExitCode::from(2);
    };
    let config = parse_config(&args[2..]);

    let before = match load(before_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let mut after = match load(after_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let mut before = before;

    // Apply ignores at diff time too, so patterns can be added after snapshots
    // were taken without recollecting.
    collect::apply_ignores(&mut before, &config);
    collect::apply_ignores(&mut after, &config);

    let changed = render_diff(&before, &after);
    if changed {
        ExitCode::FAILURE
    } else {
        println!("state-diff: no differences — the machine is unchanged.");
        ExitCode::SUCCESS
    }
}

fn cmd_facets() -> ExitCode {
    // Collect with defaults and print which facets this OS reports and whether
    // each is currently readable. Useful for setting up a run.
    let snap = collector().collect(&Config::default());
    for (name, facet) in &snap.facets {
        match &facet.error {
            Some(e) => println!("  {name:<24} unavailable: {e}"),
            None => println!(
                "  {name:<24} ok ({} lines) [{}]",
                facet.lines.len(),
                facet.source
            ),
        }
    }
    ExitCode::SUCCESS
}

fn load(path: &str) -> Result<Snapshot, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("parse {path}: {e}"))
}

/// Prints a per-facet diff. Returns true if anything differed.
fn render_diff(before: &Snapshot, after: &Snapshot) -> bool {
    let mut any = false;

    // Comparing snapshots taken on different operating systems is almost
    // certainly a mistake (wrong file picked), and the facet names barely
    // overlap, so say so rather than emitting a meaningless wall of diff.
    if before.os != after.os {
        eprintln!(
            "warning: comparing a {} snapshot against a {} snapshot",
            before.os, after.os
        );
    }

    let names: BTreeSet<&String> = before.facets.keys().chain(after.facets.keys()).collect();
    for name in names {
        let b = before.facets.get(name);
        let a = after.facets.get(name);

        // An error state changing (readable -> unreadable or vice versa) is
        // itself worth reporting: a comparison against an unavailable facet is
        // not a clean bill of health.
        let b_lines: BTreeSet<&String> = b.map(|f| f.lines.iter().collect()).unwrap_or_default();
        let a_lines: BTreeSet<&String> = a.map(|f| f.lines.iter().collect()).unwrap_or_default();

        let added: Vec<&&String> = a_lines.difference(&b_lines).collect();
        let removed: Vec<&&String> = b_lines.difference(&a_lines).collect();

        let b_err = b.and_then(|f| f.error.as_ref());
        let a_err = a.and_then(|f| f.error.as_ref());
        let err_changed = b_err != a_err;

        if added.is_empty() && removed.is_empty() && !err_changed {
            continue;
        }
        any = true;
        println!("~ {name}");
        if err_changed {
            println!("    availability changed: {b_err:?} -> {a_err:?}");
        }
        for line in removed {
            println!("  - {line}");
        }
        for line in added {
            println!("  + {line}");
        }
    }

    if any {
        println!("\nstate-diff: DIFFERENCES FOUND — the machine was left changed.");
    }
    any
}
