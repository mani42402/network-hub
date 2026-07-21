//! SPIKE-D boot-cleanup: the F-2 deadman check in miniature.
//!
//! Runs once at boot, BEFORE the main daemon, via a systemd oneshot unit. If
//! NetworkHub-owned state is present but the daemon is not going to reclaim it
//! (here: a leftover state tag with no live daemon), it removes the orphaned
//! state so the machine is never stranded by leftover firewall rules.
//!
//! In the real product this scans for NetworkHub-tagged nftables tables, routes,
//! and sysctl changes and reconciles them against persisted desired state
//! (ARCHITECTURE.md §11, §5.3). Here it is one tag file, which is enough to
//! prove the mechanism and ordering.

use std::path::Path;

const STATE_TAG: &str = "/run/networkhub-spike-d.state";
const SOCKET_PATH: &str = "/run/networkhub-spike-d.sock";

fn main() {
    eprintln!("[spike-d-cleanup] boot-time deadman check");

    // A live socket means the daemon is already up and owns its state; leave it
    // alone. On a real boot this unit runs before the daemon, so this is the
    // rare race-guard, not the common path.
    if Path::new(SOCKET_PATH).exists() && daemon_is_live() {
        eprintln!("[spike-d-cleanup] daemon is live; nothing to reclaim");
        return;
    }

    if Path::new(STATE_TAG).exists() {
        match std::fs::remove_file(STATE_TAG) {
            Ok(()) => eprintln!("[spike-d-cleanup] removed ORPHANED state tag {STATE_TAG}"),
            Err(e) => {
                eprintln!("[spike-d-cleanup] FAILED to remove {STATE_TAG}: {e}");
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("[spike-d-cleanup] no orphaned state; clean boot");
    }

    // A stale socket file left by an unclean exit is also orphaned state.
    if Path::new(SOCKET_PATH).exists() {
        let _ = std::fs::remove_file(SOCKET_PATH);
        eprintln!("[spike-d-cleanup] removed stale socket");
    }
}

/// Whether a daemon is actually listening, versus a stale socket file.
fn daemon_is_live() -> bool {
    std::os::unix::net::UnixStream::connect(SOCKET_PATH).is_ok()
}
