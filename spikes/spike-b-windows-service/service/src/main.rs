//! SPIKE-B: Windows service lifecycle for `networkhubd`.
//!
//! Questions (ARCHITECTURE.md §3.1, §10; ADR-0002):
//!   1. Install / start / stop / uninstall a Windows service from a Rust binary,
//!      with recovery actions (auto-restart on crash).
//!   2. Serve a named pipe whose ACL (via an SDDL string) admits only SYSTEM,
//!      Administrators, and a custom operators group — the ADR-0002 boundary.
//!   3. Identify the calling process on the pipe (GetNamedPipeClientProcessId),
//!      the Windows equivalent of SO_PEERCRED.
//!   4. On start, detect and clean an orphaned pipe/state from a previous run
//!      (the F-4 / F-2 analogue on Windows).
//!
//! Usage:
//!   spike-b-service install     (elevated) register the service + recovery
//!   spike-b-service uninstall   (elevated) remove it
//!   spike-b-service run         invoked by the SCM; not run by hand
//!   spike-b-service probe       connect to the pipe and print the reply
//!
//! Throwaway spike code.

#[cfg(not(windows))]
fn main() {
    eprintln!("SPIKE-B is Windows-only. Cross-compiles for type-checking; run it on Windows.");
    std::process::exit(2);
}

#[cfg(windows)]
fn main() {
    let cmd = std::env::args().nth(1).unwrap_or_default();
    let result = match cmd.as_str() {
        "install" => win::install(),
        "uninstall" => win::uninstall(),
        "run" => win::run_from_scm(),
        "probe" => win::probe(),
        other => {
            eprintln!("unknown command: {other:?}");
            eprintln!("usage: spike-b-service [install|uninstall|run|probe]");
            std::process::exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

#[cfg(windows)]
mod win {
    use std::ffi::OsString;
    use std::io::{Read, Write};
    use std::time::Duration;

    use windows_service::service::{
        ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceState,
        ServiceType,
    };
    use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

    const SERVICE_NAME: &str = "NetworkHubSpikeB";
    const PIPE_NAME: &str = r"\\.\pipe\networkhub-spike-b";

    // SDDL for the pipe's security descriptor:
    //   D:            a DACL follows
    //   (A;;GA;;;SY)  Allow, Generic All, to SYSTEM
    //   (A;;GA;;;BA)  Allow, Generic All, to Built-in Administrators
    //   (A;;GRGW;;;IU) Allow, Generic Read/Write, to Interactive Users
    // The real daemon replaces IU with the SID of a dedicated "NetworkHub
    // Operators" group (ADR-0002). IU is used here so the spike is runnable
    // without first creating a group.
    const PIPE_SDDL: &str = "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)";

    // -------- service control ------------------------------------------------

    pub fn install() -> Result<(), Box<dyn std::error::Error>> {
        let manager =
            ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)?;
        let exe = std::env::current_exe()?;
        let info = ServiceInfo {
            name: OsString::from(SERVICE_NAME),
            display_name: OsString::from("NetworkHub SPIKE-B"),
            service_type: ServiceType::OWN_PROCESS,
            start_type: ServiceStartType::AutoStart,
            error_control: ServiceErrorControl::Normal,
            executable_path: exe,
            launch_arguments: vec![OsString::from("run")],
            dependencies: vec![],
            account_name: None, // None => LocalSystem
            account_password: None,
        };
        let service = manager.create_service(
            &info,
            ServiceAccess::CHANGE_CONFIG | ServiceAccess::START,
        )?;

        // Recovery: restart the service if it crashes. This is how the daemon
        // recovers from an unexpected exit; combined with the on-start orphan
        // check, it is the Windows half of the fail-open story.
        service.update_failure_actions(windows_service::service::ServiceFailureActions {
            reset_period: windows_service::service::ServiceFailureResetPeriod::After(
                Duration::from_secs(86400),
            ),
            reboot_msg: None,
            command: None,
            actions: Some(vec![
                windows_service::service::ServiceAction {
                    action_type: windows_service::service::ServiceActionType::Restart,
                    delay: Duration::from_secs(2),
                };
                3
            ]),
        })?;

        println!("installed service {SERVICE_NAME} with restart-on-failure");
        Ok(())
    }

    pub fn uninstall() -> Result<(), Box<dyn std::error::Error>> {
        let manager =
            ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
        let service = manager.open_service(
            SERVICE_NAME,
            ServiceAccess::STOP | ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS,
        )?;

        if service.query_status()?.current_state != ServiceState::Stopped {
            service.stop()?;
            std::thread::sleep(Duration::from_secs(1));
        }
        service.delete()?;
        println!("uninstalled service {SERVICE_NAME}");
        Ok(())
    }

    // -------- SCM entry point ------------------------------------------------

    windows_service::define_windows_service!(ffi_service_main, service_main);

    pub fn run_from_scm() -> Result<(), Box<dyn std::error::Error>> {
        windows_service::service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
        Ok(())
    }

    fn service_main(_args: Vec<OsString>) {
        if let Err(e) = serve() {
            eprintln!("service failed: {e}");
        }
    }

    fn serve() -> Result<(), Box<dyn std::error::Error>> {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use windows_service::service::{
            ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceStatus,
        };
        use windows_service::service_control_handler::{self, ServiceControlHandlerResult};

        let stop = Arc::new(AtomicBool::new(false));
        let stop_h = stop.clone();
        let status_handle = service_control_handler::register(
            SERVICE_NAME,
            move |control| match control {
                ServiceControl::Stop | ServiceControl::Shutdown => {
                    stop_h.store(true, Ordering::SeqCst);
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            },
        )?;

        let running = |state, accept| ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: state,
            controls_accepted: accept,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        };

        // F-4/F-2 analogue: reclaim orphaned state from a previous instance
        // before announcing Running. On Windows a named pipe vanishes with its
        // owning process, so a *stale state file* is the orphan we clean.
        clean_orphans();

        status_handle.set_service_status(running(
            ServiceState::Running,
            ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        ))?;

        // Serve the pipe until asked to stop.
        while !stop.load(Ordering::SeqCst) {
            if let Err(e) = accept_one() {
                eprintln!("pipe error: {e}");
            }
        }

        status_handle.set_service_status(running(ServiceState::Stopped, ServiceControlAccept::empty()))?;
        Ok(())
    }

    // -------- named pipe with SDDL ------------------------------------------

    fn accept_one() -> Result<(), Box<dyn std::error::Error>> {
        use std::os::windows::io::FromRawHandle;
        use windows_sys::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
        use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
        // PIPE_ACCESS_DUPLEX is classified under Storage::FileSystem in
        // windows-sys 0.61, not System::Pipes.
        use windows_sys::Win32::Storage::FileSystem::PIPE_ACCESS_DUPLEX;
        use windows_sys::Win32::System::Pipes::{
            ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeClientProcessId, PIPE_READMODE_BYTE,
            PIPE_TYPE_BYTE, PIPE_WAIT,
        };

        // Build a SECURITY_ATTRIBUTES from the SDDL string.
        let sddl: Vec<u16> = PIPE_SDDL.encode_utf16().chain([0]).collect();
        let mut psd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        // SAFETY: sddl is a valid null-terminated UTF-16 string; psd receives an
        // allocation we free below.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                1, // SDDL_REVISION_1
                &raw mut psd,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err("ConvertStringSecurityDescriptor failed".into());
        }

        let mut sa = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: psd,
            bInheritHandle: 0,
        };

        let name: Vec<u16> = PIPE_NAME.encode_utf16().chain([0]).collect();
        // SAFETY: name is null-terminated; sa is valid; other args are literals.
        let handle = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                1,
                4096,
                4096,
                0,
                &raw mut sa,
            )
        };
        // The security descriptor is copied into the pipe; free our copy.
        // SAFETY: psd was allocated by the Convert call above.
        unsafe { windows_sys::Win32::Foundation::LocalFree(psd as _) };

        if handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            return Err("CreateNamedPipeW failed".into());
        }

        // SAFETY: waiting for a client on a valid pipe handle.
        let connected = unsafe { ConnectNamedPipe(handle, std::ptr::null_mut()) };
        let _ = connected; // ERROR_PIPE_CONNECTED is also success; ignored for the spike.

        // Windows equivalent of peer credentials.
        let mut pid: u32 = 0;
        // SAFETY: handle valid, pid is a valid out-pointer.
        unsafe { GetNamedPipeClientProcessId(handle, &raw mut pid) };

        // SAFETY: adopt the pipe handle as a File for convenient I/O.
        let mut file = unsafe { std::fs::File::from_raw_handle(handle as _) };
        let reply = format!("spike-b service alive; client pid={pid}\n");
        let _ = file.write_all(reply.as_bytes());
        let mut buf = [0u8; 64];
        let _ = file.read(&mut buf);
        Ok(())
    }

    fn clean_orphans() {
        let state = std::env::temp_dir().join("networkhub-spike-b.state");
        if state.exists() {
            let _ = std::fs::remove_file(&state);
            eprintln!("[spike-b] removed orphaned state {}", state.display());
        }
        let _ = std::fs::write(
            std::env::temp_dir().join("networkhub-spike-b.state"),
            b"owned",
        );
    }

    // -------- probe ----------------------------------------------------------

    pub fn probe() -> Result<(), Box<dyn std::error::Error>> {
        use std::fs::OpenOptions;
        // A named pipe can be opened as a file by the client side.
        let mut pipe = OpenOptions::new().read(true).write(true).open(PIPE_NAME)?;
        pipe.write_all(b"PING\n")?;
        let mut buf = [0u8; 128];
        let n = pipe.read(&mut buf)?;
        println!("reply: {}", String::from_utf8_lossy(&buf[..n]).trim());
        println!("SPIKE-B probe: PASS");
        Ok(())
    }
}
