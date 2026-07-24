//! Service-manager readiness signalling (§4.2).
//!
//! A `Type=notify` systemd unit does not consider the service *started* until it
//! signals readiness, so dependent units order correctly and `systemctl start`
//! blocks until the daemon is actually serving. We signal this ourselves by
//! writing `READY=1` to the socket named in `$NOTIFY_SOCKET` — a stable,
//! documented protocol — rather than taking a libsystemd dependency (the same
//! approach proven in SPIKE-D).
//!
//! On macOS launchd needs no readiness signal, and on Windows the SCM readiness
//! is reported from the service wrapper (`service` module); on those platforms
//! this is a no-op.

/// Signals readiness to the service manager, if one is expecting it.
///
/// Best-effort: absence of `$NOTIFY_SOCKET` (the daemon was not launched by a
/// `Type=notify` unit) is normal and silent.
pub fn notify_ready() {
    #[cfg(target_os = "linux")]
    {
        if let Err(e) = sd_notify_ready() {
            tracing::debug!("sd_notify skipped: {e}");
        }
    }
}

#[cfg(target_os = "linux")]
fn sd_notify_ready() -> Result<(), String> {
    use std::os::unix::net::UnixDatagram;

    let path = std::env::var("NOTIFY_SOCKET").map_err(|_| "NOTIFY_SOCKET unset".to_owned())?;

    // Abstract-namespace sockets (leading '@') would need a raw sockaddr to
    // address from std; the pathful form is what our own unit uses. Handle the
    // common case and skip the abstract one honestly.
    if path.starts_with('@') {
        return Err("abstract NOTIFY_SOCKET is not supported by this notifier".to_owned());
    }

    let socket = UnixDatagram::unbound().map_err(|e| e.to_string())?;
    socket
        .send_to(b"READY=1", &path)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn notify_ready_does_not_panic() {
        // Whether or not a service manager is present, this must be safe to call.
        // It reads $NOTIFY_SOCKET (best-effort) and never panics; env is not
        // mutated here because std::env::set_var is unsafe under edition 2024.
        super::notify_ready();
    }
}
