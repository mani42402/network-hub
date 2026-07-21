//! Repository automation tasks. Run via `cargo xtask <command>`.
//!
//! The headline task is `check-layers`, which turns the dependency rules in
//! `docs/architecture/ARCHITECTURE.md` §5.1 from prose into a build failure.
//! Architecture that is not mechanically enforced decays; this is the enforcer.

use std::process::{Command, ExitCode};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// Concrete Platform Adapter Layer backends. Feature crates must depend on the
/// `nh-pal` traits instead, so that business logic stays OS-agnostic and
/// testable against `pal-mock` (AD-7).
const PAL_BACKENDS: &[&str] = &["pal-linux", "pal-windows", "pal-macos"];

/// The only crate permitted to select a concrete PAL backend.
const PAL_BACKEND_CONSUMER: &str = "nh-daemon";

/// The only crates permitted to produce executables. `xtask` is repo tooling
/// rather than product code, and is exempt from the product layer rules.
const BINARY_CRATES: &[&str] = &["nh-daemon", "nh-gateway", "nh-cli", "xtask"];

/// The foundation crate: shared vocabulary, no internal dependencies at all.
const FOUNDATION_CRATE: &str = "nh-types";

fn main() -> ExitCode {
    let command = std::env::args().nth(1);
    let result = match command.as_deref() {
        Some("check-layers") => check_layers(),
        Some(other) => {
            eprintln!("xtask: unknown command `{other}`");
            usage();
            return ExitCode::FAILURE;
        }
        None => {
            usage();
            return ExitCode::FAILURE;
        }
    };

    match result {
        Ok(violations) if violations.is_empty() => {
            println!("xtask check-layers: OK — all architecture layer rules hold.");
            ExitCode::SUCCESS
        }
        Ok(violations) => {
            eprintln!(
                "xtask check-layers: {} violation(s) of the dependency rules in\n\
                 docs/architecture/ARCHITECTURE.md §5.1:\n",
                violations.len()
            );
            for v in &violations {
                eprintln!("  ✗ {v}");
            }
            eprintln!(
                "\nThese rules exist so the engine stays portable and testable. If a rule\n\
                 is genuinely wrong, change it in an ADR first — not in a hurry."
            );
            ExitCode::FAILURE
        }
        Err(err) => {
            eprintln!("xtask: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn usage() {
    eprintln!(
        "usage: cargo xtask <command>\n\n\
         commands:\n  \
         check-layers    Enforce the crate dependency rules (ARCHITECTURE.md §5.1)"
    );
}

// ---------------------------------------------------------------------------
// cargo metadata model — only the fields we actually read.
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    dependencies: Vec<Dependency>,
    targets: Vec<Target>,
}

#[derive(Deserialize)]
struct Dependency {
    name: String,
    /// `None` for a normal dependency, `Some("dev")` / `Some("build")` otherwise.
    kind: Option<String>,
    /// Present only for path dependencies, which is how we identify
    /// workspace-internal edges without resolving the whole graph.
    path: Option<String>,
}

#[derive(Deserialize)]
struct Target {
    kind: Vec<String>,
}

impl Dependency {
    /// A dependency on another crate in this repository.
    fn is_internal(&self) -> bool {
        self.path.is_some()
    }

    /// Dev-dependencies are exempt from the PAL rule: a test may legitimately
    /// exercise a concrete backend. They are never exempt from the foundation
    /// rule, because `nh-types` must stay a leaf in every configuration.
    fn is_dev(&self) -> bool {
        self.kind.as_deref() == Some("dev")
    }
}

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

fn check_layers() -> Result<Vec<String>> {
    let metadata = load_metadata()?;
    let mut violations = Vec::new();

    for package in &metadata.packages {
        // Only judge crates that belong to this workspace.
        if !metadata.workspace_members.contains(&package.id) {
            continue;
        }

        rule_foundation_is_a_leaf(package, &mut violations);
        rule_only_daemon_picks_a_backend(package, &mut violations);
        rule_binaries_are_declared(package, &mut violations);
        rule_daemon_is_a_leaf(package, &mut violations);
    }

    Ok(violations)
}

/// Rule 1 — `nh-types` depends on nothing internal. Everything may depend on it.
fn rule_foundation_is_a_leaf(package: &Package, violations: &mut Vec<String>) {
    if package.name != FOUNDATION_CRATE {
        return;
    }
    for dep in package.dependencies.iter().filter(|d| d.is_internal()) {
        violations.push(format!(
            "{FOUNDATION_CRATE} depends on `{}`. The foundation crate must have no \
             internal dependencies so every other crate can depend on it freely.",
            dep.name
        ));
    }
}

/// Rule 2 — feature crates depend on the `nh-pal` traits, never on a concrete
/// backend. Only `nh-daemon` wires a real backend in.
fn rule_only_daemon_picks_a_backend(package: &Package, violations: &mut Vec<String>) {
    if package.name == PAL_BACKEND_CONSUMER {
        return;
    }
    for dep in &package.dependencies {
        if PAL_BACKENDS.contains(&dep.name.as_str()) && !dep.is_dev() {
            violations.push(format!(
                "`{}` depends on the concrete backend `{}`. Depend on the `nh-pal` \
                 traits instead; only `{PAL_BACKEND_CONSUMER}` selects a backend (AD-7).",
                package.name, dep.name
            ));
        }
    }
}

/// Rule 3 — only the three product binaries (plus `xtask`) produce executables.
fn rule_binaries_are_declared(package: &Package, violations: &mut Vec<String>) {
    let has_bin = package
        .targets
        .iter()
        .any(|t| t.kind.iter().any(|k| k == "bin"));

    if has_bin && !BINARY_CRATES.contains(&package.name.as_str()) {
        violations.push(format!(
            "`{}` produces a binary target. Only {} may do so; everything else is a \
             library so it stays testable and reusable.",
            package.name,
            BINARY_CRATES.join(", ")
        ));
    }
}

/// Rule 4 — nothing depends on the daemon. It is the composition root, and a
/// dependency on it would smuggle process-lifecycle concerns into a library.
fn rule_daemon_is_a_leaf(package: &Package, violations: &mut Vec<String>) {
    if package.name == PAL_BACKEND_CONSUMER {
        return;
    }
    for dep in &package.dependencies {
        if dep.name == PAL_BACKEND_CONSUMER && !dep.is_dev() {
            violations.push(format!(
                "`{}` depends on `{PAL_BACKEND_CONSUMER}`. The daemon is the composition \
                 root and must remain a leaf of the dependency graph.",
                package.name
            ));
        }
    }
}

// ---------------------------------------------------------------------------

fn load_metadata() -> Result<Metadata> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .context("failed to run `cargo metadata`")?;

    if !output.status.success() {
        bail!(
            "`cargo metadata` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    serde_json::from_slice(&output.stdout).context("failed to parse `cargo metadata` output")
}
