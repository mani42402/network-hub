//! SPIKE-C helper: a stand-in for `networkhubd` on macOS.
//!
//! Question this answers: can we ship a privileged root daemon on macOS that a
//! user installs, that survives reboot, and that uninstalls cleanly? The real
//! product registers it via `SMAppService` from a signed app bundle; the
//! mechanism underneath is a launchd LaunchDaemon, which is what this exercises.
//!
//! The helper listens on a Unix domain socket and answers a one-line protocol:
//!   PING        -> PONG
//!   WHOAMI      -> euid, uid, and whether it is running as root
//!   PRIVCHECK   -> whether it can read a root-only path (proves real privilege)
//!
//! Throwaway spike code. No dependencies, standard library only.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

// A system location, not a per-user one: a LaunchDaemon runs before and outside
// any login session. `/var/run` is the conventional home for daemon sockets.
// Overridable so the round trip can be exercised without root, which
// cannot write /var/run. Under launchd the daemon uses the default.
fn socket_path() -> String {
    std::env::var("SPIKE_C_SOCKET")
        .unwrap_or_else(|_| "/var/run/networkhub-spike-c.sock".to_owned())
}

fn main() {
    let path = socket_path();
    // launchd expects the daemon to run in the foreground; it owns the process
    // lifecycle. Daemonising ourselves would fight it.
    eprintln!("[spike-c-helper] starting, pid={}", std::process::id());

    // A stale socket from an unclean previous exit would block bind — a
    // miniature of the daemon's real deadman problem (ARCHITECTURE.md §11).
    if Path::new(&path).exists() {
        let _ = std::fs::remove_file(&path);
        eprintln!("[spike-c-helper] removed stale socket");
    }

    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[spike-c-helper] bind {path} failed: {e}");
            std::process::exit(1);
        }
    };

    // 0666 for the spike so an unprivileged probe can connect and read the
    // helper's reported euid. The real daemon restricts this to a group via
    // peer-credential auth (ADR-0002); here the point is only to observe that
    // launchd started us as root.
    if let Err(e) = set_socket_mode(&path, 0o666) {
        eprintln!("[spike-c-helper] chmod failed: {e}");
    }

    eprintln!("[spike-c-helper] listening on {path}, euid={}", geteuid());

    for stream in listener.incoming() {
        match stream {
            Ok(s) => handle(s),
            Err(e) => eprintln!("[spike-c-helper] accept error: {e}"),
        }
    }
}

fn handle(stream: UnixStream) {
    let peer = stream.try_clone();
    let reader = BufReader::new(stream);
    let Ok(mut writer) = peer else { return };

    for line in reader.lines() {
        let Ok(line) = line else { break };
        let response = match line.trim() {
            "PING" => "PONG".to_owned(),
            "WHOAMI" => {
                let euid = geteuid();
                format!("euid={euid} uid={} root={}", getuid(), euid == 0)
            }
            "PRIVCHECK" => {
                // Reading a root-only file proves real privilege rather than a
                // claimed euid. /var/db/sudo is 0700 root:wheel on macOS.
                match std::fs::read_dir("/var/db/sudo") {
                    Ok(_) => "privcheck=ok can-read-root-only-path".to_owned(),
                    Err(e) => format!("privcheck=denied {e}"),
                }
            }
            other => format!("ERR unknown command: {other}"),
        };
        if writeln!(writer, "{response}").is_err() {
            break;
        }
    }
}

fn set_socket_mode(path: &str, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

// Minimal libc bindings. Pulling in the `libc` crate for two calls would be more
// ceremony than the spike warrants.
fn geteuid() -> u32 {
    // SAFETY: geteuid is always safe; it takes no arguments and cannot fail.
    unsafe { libc_geteuid() }
}
fn getuid() -> u32 {
    // SAFETY: getuid is always safe; it takes no arguments and cannot fail.
    unsafe { libc_getuid() }
}

unsafe extern "C" {
    #[link_name = "geteuid"]
    fn libc_geteuid() -> u32;
    #[link_name = "getuid"]
    fn libc_getuid() -> u32;
}
