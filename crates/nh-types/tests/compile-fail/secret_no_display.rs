//! A `Secret` must not be printable with `Display` (`{}`). If this ever
//! compiles, the no-leak guarantee has been broken.

use nh_types::Secret;

fn main() {
    let secret = Secret::new(String::from("hunter2"));
    // `Secret` deliberately has no `Display` impl, so this must not compile.
    println!("{}", secret);
}
