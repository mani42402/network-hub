//! SPIKE-D daemon: a stand-in for `networkhubd` under systemd on Linux.
//!
//! Questions this answers (ARCHITECTURE.md §3.2, §10.6, §11; ADR-0005):
//!   1. Does the daemon run under a hardened systemd unit (ProtectSystem=strict
//!      and friends) and still function?
//!   2. Can it signal readiness with sd_notify so `systemctl start` blocks until
//!      it is actually up?
//!   3. Does the boot-cleanup companion remove orphaned NetworkHub state if the
//!      daemon does not come back — the F-2 safety net for a stranded network?
//!
//! To make (3) concrete without touching real firewall rules, the daemon writes
//! a "tag file" representing NetworkHub-owned OS state (stand-in for an nftables
//! table or a sysctl change). The cleanup unit removes any such tag on boot when
//! the daemon is not running, exactly as the real deadman check will remove
//! orphaned firewall state.
//!
//! Throwaway spike code, standard library only.

use std::io::Write;
use std::os::unix::net::{UnixDatagram, UnixListener};
use std::path::Path;

const SOCKET_PATH: &str = "/run/networkhub-spike-d.sock";
// Stand-in for NetworkHub-owned OS state that must never be orphaned.
const STATE_TAG: &str = "/run/networkhub-spike-d.state";

fn main() {
    eprintln!("[spike-d] starting, pid={}", std::process::id());

    // Represent "we have applied some network state". If we die and do not come
    // back, the boot-cleanup unit must remove this — otherwise, in the real
    // product, it would be a leftover firewall rule stranding the user.
    if let Err(e) = std::fs::write(STATE_TAG, b"owned-by-networkhub\n") {
        eprintln!("[spike-d] could not write state tag: {e}");
    } else {
        eprintln!("[spike-d] applied state, tagged at {STATE_TAG}");
    }

    if Path::new(SOCKET_PATH).exists() {
        let _ = std::fs::remove_file(SOCKET_PATH);
    }
    let listener = match UnixListener::bind(SOCKET_PATH) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[spike-d] bind failed: {e}");
            std::process::exit(1);
        }
    };

    // Tell systemd we are ready. With Type=notify the unit's `start` job does
    // not complete until this fires, so dependent services order correctly.
    notify_ready();
    eprintln!("[spike-d] READY (notified systemd)");

    // Clean teardown on SIGTERM (what `systemctl stop` sends): remove our state,
    // the graceful-exit counterpart to the boot-cleanup deadman.
    install_sigterm_handler();

    for stream in listener.incoming() {
        match stream {
            Ok(mut s) => {
                let _ = writeln!(s, "spike-d alive, pid={}", std::process::id());
            }
            Err(e) => eprintln!("[spike-d] accept error: {e}"),
        }
    }
}

/// sd_notify(READY=1) without the libsystemd dependency: write the datagram to
/// the socket named in $NOTIFY_SOCKET ourselves. This is a documented, stable
/// protocol.
fn notify_ready() {
    let Ok(path) = std::env::var("NOTIFY_SOCKET") else {
        eprintln!("[spike-d] NOTIFY_SOCKET unset (not launched by systemd Type=notify)");
        return;
    };
    match UnixDatagram::unbound() {
        Ok(sock) => {
            // Abstract-namespace sockets start with '@' in NOTIFY_SOCKET.
            let target = if let Some(stripped) = path.strip_prefix('@') {
                let mut p = String::from("\0");
                p.push_str(stripped);
                p
            } else {
                path
            };
            if let Err(e) = sock.send_to(b"READY=1", &target) {
                eprintln!("[spike-d] sd_notify failed: {e}");
            }
        }
        Err(e) => eprintln!("[spike-d] could not open notify socket: {e}"),
    }
}

fn install_sigterm_handler() {
    // SAFETY: signal() installs a handler for SIGTERM. The handler only calls
    // async-signal-safe functions (unlink, _exit).
    unsafe {
        libc_signal(15 /* SIGTERM */, sigterm_handler as *const () as usize);
    }
}

extern "C" fn sigterm_handler(_sig: i32) {
    // SAFETY: unlink and _exit are async-signal-safe. Paths are static C strings.
    // c"..." literals are *const u8; cast to the platform's c_char (i8 on
    // x86-64, u8 on aarch64 — `c_char` is the correct alias for both).
    unsafe {
        libc_unlink(c"/run/networkhub-spike-d.state".as_ptr().cast::<c_char>());
        libc_unlink(c"/run/networkhub-spike-d.sock".as_ptr().cast::<c_char>());
        libc_exit(0);
    }
}

// Minimal libc surface. The real daemon uses the `libc` crate behind the PAL.
use std::os::raw::c_char;

unsafe extern "C" {
    #[link_name = "signal"]
    fn libc_signal(sig: i32, handler: usize) -> usize;
    #[link_name = "unlink"]
    fn libc_unlink(path: *const c_char) -> i32;
    #[link_name = "_exit"]
    fn libc_exit(code: i32) -> !;
}
