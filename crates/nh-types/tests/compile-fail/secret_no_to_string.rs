//! `.to_string()` comes from `Display`, so it must also be unavailable on a
//! `Secret` — another route by which a secret could reach a string and then a
//! log.

use nh_types::Secret;

fn main() {
    let secret = Secret::new(String::from("hunter2"));
    // No `Display` ⇒ no `ToString` ⇒ this must not compile.
    let _leaked: String = secret.to_string();
}
