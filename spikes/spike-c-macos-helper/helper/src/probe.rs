//! SPIKE-C probe: an unprivileged client that queries the helper and reports
//! what it found. Stands in for `nh-cli` talking to the daemon.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

// Overridable so the round trip can be exercised without root, which
// cannot write /var/run. Under launchd the daemon uses the default.
fn socket_path() -> String {
    std::env::var("SPIKE_C_SOCKET")
        .unwrap_or_else(|_| "/var/run/networkhub-spike-c.sock".to_owned())
}

fn main() {
    let path = socket_path();
    let mut failures = 0;

    let Ok(stream) = UnixStream::connect(&path) else {
        eprintln!("FAIL: cannot connect to {path} — is the helper running?");
        std::process::exit(1);
    };
    let mut writer = stream.try_clone().expect("clone stream");
    let mut reader = BufReader::new(stream);

    for (cmd, check) in [
        ("PING", "PONG"),
        ("WHOAMI", "root="),
        ("PRIVCHECK", "privcheck="),
    ] {
        writeln!(writer, "{cmd}").expect("write");
        let mut line = String::new();
        reader.read_line(&mut line).expect("read");
        let line = line.trim();
        let ok = line.contains(check);
        println!("  {cmd:<10} -> {line}   [{}]", if ok { "ok" } else { "FAIL" });
        if !ok {
            failures += 1;
        }

        // The headline of the spike: is the helper actually root? When launchd
        // starts it as a LaunchDaemon it should be; when a developer runs it by
        // hand it will not be, and the probe says so plainly.
        if cmd == "WHOAMI" {
            if line.contains("root=true") {
                println!("             => helper is running as ROOT (launchd path)");
            } else {
                println!("             => helper is NOT root (started as a normal user)");
            }
        }
    }

    if failures == 0 {
        println!("SPIKE-C probe: PASS");
    } else {
        println!("SPIKE-C probe: FAIL ({failures})");
        std::process::exit(1);
    }
}
