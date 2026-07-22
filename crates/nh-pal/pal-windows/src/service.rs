//! Windows Service (SCM) integration for the daemon (§3.1, §10; ADR-0002).
//!
//! Registers `networkhubd` as an `OWN_PROCESS` service that starts at boot,
//! **restarts on crash** (recovery actions), and runs its on-start orphan check
//! before reporting `Running` — the Windows analogue of the boot-cleanup deadman
//! (F-2). The daemon supplies the actual work as a body closure to [`run`]; this
//! module owns only the SCM handshake.
//!
//! **Runtime verification is hardware-gated** (there is no SCM in CI): this
//! compiles and lints on Windows, and is exercised on the Windows box via the
//! install → reboot → kill → recover protocol in `docs/ops/service.md`.

use std::ffi::OsString;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use windows_service::service::{
    ServiceAccess, ServiceAction, ServiceActionType, ServiceControl, ServiceControlAccept,
    ServiceErrorControl, ServiceExitCode, ServiceFailureActions, ServiceFailureResetPeriod,
    ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

/// The registered service name (the SCM key, and the argument to `sc.exe`).
pub const SERVICE_NAME: &str = "networkhubd";

/// The human-readable name shown in `services.msc`.
pub const DISPLAY_NAME: &str = "NetworkHub";

/// A boxed error from an SCM operation.
type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// A stop request raised by the SCM. The daemon awaits [`StopToken::wait`] as
/// its shutdown future, so a `services.msc` stop (or system shutdown) drives the
/// same graceful teardown as Ctrl-C does on Unix.
#[derive(Clone, Debug)]
pub struct StopToken {
    flag: Arc<AtomicBool>,
    notify: Arc<tokio::sync::Notify>,
}

impl StopToken {
    fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Signals a stop. Safe to call from the SCM control-handler thread, which
    /// has no tokio runtime: `notify_waiters` does not require one.
    fn signal(&self) {
        self.flag.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    /// Resolves once the SCM has requested a stop. The `notified()`-before-check
    /// ordering avoids the lost-wakeup race with [`StopToken::signal`].
    pub async fn wait(&self) {
        loop {
            let notified = self.notify.notified();
            if self.flag.load(Ordering::SeqCst) {
                return;
            }
            notified.await;
        }
    }
}

// ---------------------------------------------------------------------------
// Service control (install / uninstall), run elevated.
// ---------------------------------------------------------------------------

/// Registers the service with the SCM: auto-start at boot as `LocalSystem`,
/// launched with the given `launch_args` (e.g. `["service"]`), and configured to
/// restart on failure. Requires elevation.
///
/// # Errors
/// Returns an error if the current executable path cannot be determined or the
/// SCM rejects the create/config calls (typically: not elevated, or already
/// installed).
pub fn install(launch_args: &[&str]) -> Result<(), BoxError> {
    let manager =
        ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)?;
    let executable_path = std::env::current_exe()?;

    let info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path,
        launch_arguments: launch_args.iter().map(OsString::from).collect(),
        dependencies: vec![],
        account_name: None, // None => LocalSystem
        account_password: None,
    };

    let service =
        manager.create_service(&info, ServiceAccess::CHANGE_CONFIG | ServiceAccess::START)?;

    // Recovery: restart the daemon if it exits unexpectedly. With the on-start
    // orphan check, this is the Windows half of the fail-open story — a crashed
    // daemon comes back, and reclaims what the crash left behind, automatically.
    service.update_failure_actions(ServiceFailureActions {
        reset_period: ServiceFailureResetPeriod::After(Duration::from_hours(24)),
        reboot_msg: None,
        command: None,
        actions: Some(vec![
            ServiceAction {
                action_type: ServiceActionType::Restart,
                delay: Duration::from_secs(2),
            };
            3
        ]),
    })?;

    Ok(())
}

/// Stops (if running) and removes the service. Requires elevation. Idempotent
/// enough for an uninstall script: a not-installed service surfaces as an error
/// the caller may ignore.
///
/// # Errors
/// Returns an error if the SCM rejects the open/stop/delete calls.
pub fn uninstall() -> Result<(), BoxError> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let service = manager.open_service(
        SERVICE_NAME,
        ServiceAccess::STOP | ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS,
    )?;

    if service.query_status()?.current_state != ServiceState::Stopped {
        // Best-effort stop; delete still proceeds if the stop races.
        let _ = service.stop();
        std::thread::sleep(Duration::from_secs(1));
    }
    service.delete()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// SCM dispatch — invoked by the service manager, not by hand.
// ---------------------------------------------------------------------------

/// The daemon body, stashed for the SCM-invoked [`service_main`] to pick up.
/// A `FnOnce` because the service runs its body exactly once per process.
type ServiceBody = Box<dyn FnOnce(StopToken) + Send>;

static BODY: std::sync::Mutex<Option<ServiceBody>> = std::sync::Mutex::new(None);

windows_service::define_windows_service!(ffi_service_main, service_main);

/// Runs the daemon under SCM control. The SCM invokes `body` on its own thread
/// with a [`StopToken`]; `body` should build its runtime, perform the on-start
/// orphan check, serve until `StopToken::wait` resolves, then return.
///
/// This blocks until the service stops. It is what `nh-daemon service` calls.
///
/// # Errors
/// Returns an error if the SCM dispatcher cannot be started (e.g. the process
/// was not launched by the SCM).
pub fn run<F>(body: F) -> Result<(), BoxError>
where
    F: FnOnce(StopToken) + Send + 'static,
{
    // Stash the body where the C-ABI `service_main` can reach it. `run` is
    // called once per process, so a prior value would be a programming error.
    if let Ok(mut slot) = BODY.lock() {
        *slot = Some(Box::new(body));
    }
    windows_service::service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
    Ok(())
}

/// The Rust entry point the generated `ffi_service_main` calls. Runs on a thread
/// the SCM owns; a panic here would be lost, so failures are logged, not
/// unwound.
fn service_main(_args: Vec<OsString>) {
    if let Err(e) = serve() {
        tracing::error!(error = %e, "windows service dispatch failed");
    }
}

fn serve() -> Result<(), BoxError> {
    let stop = StopToken::new();
    let stop_for_handler = stop.clone();

    let status_handle =
        service_control_handler::register(SERVICE_NAME, move |control| match control {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                stop_for_handler.signal();
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        })?;

    let status = |state, accept| ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: accept,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };

    status_handle.set_service_status(status(
        ServiceState::StartPending,
        ServiceControlAccept::empty(),
    ))?;

    // Hand the daemon body the stop token and report Running. The body performs
    // its own on-start orphan check before it begins serving.
    let body = BODY.lock().ok().and_then(|mut slot| slot.take());

    status_handle.set_service_status(status(
        ServiceState::Running,
        ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
    ))?;

    if let Some(body) = body {
        body(stop); // blocks until the daemon has drained and returned.
    }

    status_handle
        .set_service_status(status(ServiceState::Stopped, ServiceControlAccept::empty()))?;
    Ok(())
}
