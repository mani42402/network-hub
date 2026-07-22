//! Child-process supervision primitives (F-4).
//!
//! The daemon will supervise child processes — `hostapd`, `openvpn`, the plugin
//! host — and the one thing that must never happen is an **orphan**: a child that
//! outlives the daemon, leaving a ghost access point or VPN. The machinery to
//! prevent that lands here now, before any real child exists, so every child is
//! spawned through it.
//!
//! On Unix the child is placed in its own process group so the whole group can be
//! signalled (killing descendants too), and it is killed on stop/drop. This uses
//! tokio's safe `process_group` and the `nix` crate's safe `kill`, so the crate
//! keeps `unsafe_code = "forbid"`. The Windows equivalent (a Job Object with
//! `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) lands in WP-M0.7; the API is fixed here.

use std::process::Stdio;

use tokio::process::{Child, Command};

/// A supervised child process. Killed on [`SupervisedChild::stop`] or on drop, so
/// it cannot outlive the daemon.
#[derive(Debug)]
pub struct SupervisedChild {
    name: String,
    child: Option<Child>,
}

impl SupervisedChild {
    /// Spawns `program` with `args` under supervision.
    ///
    /// On Unix the child is made its own process-group leader so a group signal
    /// reaches everything it spawns.
    ///
    /// # Errors
    /// Returns the spawn error if the process could not be started.
    pub fn spawn(name: impl Into<String>, program: &str, args: &[&str]) -> std::io::Result<Self> {
        let name = name.into();
        let mut cmd = Command::new(program);
        cmd.args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            // Kill the OS process if this handle is dropped without an explicit
            // stop — a backstop under the group-kill in `stop`.
            .kill_on_drop(true);

        // Put the child in its own process group (leader = the child). This is
        // the safe replacement for a setsid pre_exec hook.
        #[cfg(unix)]
        cmd.process_group(0);

        let child = cmd.spawn()?;
        Ok(Self {
            name,
            child: Some(child),
        })
    }

    /// The supervised child's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The OS process id, or `None` if it has already exited.
    #[must_use]
    pub fn pid(&self) -> Option<u32> {
        self.child.as_ref().and_then(Child::id)
    }

    /// Whether the process is still running (has not been reaped).
    pub fn is_running(&mut self) -> bool {
        match self.child.as_mut() {
            None => false,
            // try_wait returns Ok(None) while still running.
            Some(child) => matches!(child.try_wait(), Ok(None)),
        }
    }

    /// Stops the child, killing its process group on Unix so descendants die too,
    /// then reaps it.
    ///
    /// # Errors
    /// Returns an error only if reaping fails.
    pub async fn stop(&mut self) -> std::io::Result<()> {
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };

        #[cfg(unix)]
        if let Some(pid) = child.id() {
            // Signal the whole process group (negative pid) so anything the child
            // spawned goes too — the F-4 guarantee. ESRCH (already gone) is fine.
            if let Ok(pid) = i32::try_from(pid) {
                let _ = nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(-pid),
                    nix::sys::signal::Signal::SIGTERM,
                );
            }
        }

        child.start_kill()?;
        let _ = child.wait().await;
        Ok(())
    }
}

#[cfg(all(test, unix))]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Whether a pid still exists (signal 0 is an existence check).
    fn pid_exists(pid: u32) -> bool {
        nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(i32::try_from(pid).unwrap_or(i32::MAX)),
            None,
        )
        .is_ok()
    }

    #[tokio::test]
    async fn spawned_child_is_killed_on_stop() {
        let mut child = SupervisedChild::spawn("sleeper", "sleep", &["60"]).unwrap();
        let pid = child.pid().expect("has a pid");
        assert!(child.is_running());

        child.stop().await.unwrap();
        assert!(
            !pid_exists(pid),
            "child pid {pid} should be gone after stop"
        );
    }

    #[tokio::test]
    async fn child_is_killed_on_drop() {
        let pid = {
            let child = SupervisedChild::spawn("sleeper", "sleep", &["60"]).unwrap();
            child.pid().expect("has a pid")
        }; // dropped here — kill_on_drop terminates it
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        assert!(
            !pid_exists(pid),
            "child pid {pid} should be gone after drop"
        );
    }
}
