//! [`Secret<T>`] — a wrapper that makes leaking a secret a *compile error*.
//!
//! NetworkHub handles material that would burn a user if it appeared in a log, a
//! crash report, or a support bundle: VPN private keys, Wi-Fi passphrases, API
//! key material, token-signing keys. The engineering rule (CONTRIBUTING.md,
//! ARCHITECTURE.md §1.6) is that these are typed as `Secret<T>`, and that the
//! guarantee is enforced by the compiler rather than by reviewer vigilance.
//!
//! Two properties make that true:
//!
//! 1. **No `Display`, and a redacting `Debug`.** `format!("{secret}")` does not
//!    compile; `format!("{secret:?}")` prints `Secret(<redacted>)`. So a secret
//!    cannot reach a log line or an error string by accident — the usual way
//!    secrets escape.
//! 2. **Zeroized on drop.** The inner bytes are overwritten when the `Secret` is
//!    dropped, so a key does not linger in freed memory to be read by a later
//!    allocation or captured in a core dump.
//!
//! Reading the secret is deliberately awkward: [`Secret::expose`] is the only
//! way in, and its name is greppable, so every point where a secret is actually
//! used is visible in review.

use std::fmt;

use zeroize::Zeroize;

/// A value whose contents must never be logged, displayed, or serialized by
/// accident. See the module docs for the guarantees.
///
/// The inner type must be [`Zeroize`], which every real secret already is:
/// `String`, `Vec<u8>`, and fixed-size byte arrays all implement it. Requiring
/// it here means a `Secret` is always memory-hygienic, not only when someone
/// remembered to make it so.
pub struct Secret<T: Zeroize> {
    inner: T,
}

impl<T: Zeroize> Secret<T> {
    /// Wraps a value as a secret.
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }

    /// Borrows the underlying secret. This is the single, deliberately-named
    /// entry point for reading a secret: grep for `.expose(` to find every use.
    pub fn expose(&self) -> &T {
        &self.inner
    }

    /// Mutably borrows the underlying secret.
    pub fn expose_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

// Note: there is deliberately no `into_exposed` / owned accessor. Moving the
// value out past our zeroizing `Drop` would need either `unsafe` (forbidden in
// this crate) or `mem::forget` (a denied lint). A caller that needs ownership
// writes `secret.expose().clone()`, which leaves the original to be zeroized —
// the small clone is a fair price for keeping the hygiene guarantee airtight.

impl<T: Zeroize> Drop for Secret<T> {
    fn drop(&mut self) {
        self.inner.zeroize();
    }
}

// A redacting Debug — present so that a struct deriving Debug that contains a
// Secret still compiles and still cannot leak. Note there is deliberately NO
// Display impl; that is what makes `format!("{secret}")` fail to compile.
impl<T: Zeroize> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

impl<T: Zeroize> From<T> for Secret<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

// Constructing a secret by cloning a borrowed value is a common need (e.g. from
// a parsed config). Cloning the *inner* value is fine; what must not exist is a
// way to read it back out without `expose`.
impl<T: Zeroize + Clone> Clone for Secret<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

// Equality by exposing both sides is occasionally needed (comparing a supplied
// password to a stored one). Provided in constant time is out of scope for this
// type; callers comparing authentication material must use a constant-time
// comparison on the exposed bytes. This impl is a convenience for non-security
// equality only and is therefore NOT derived to avoid accidental use — callers
// opt in explicitly by exposing.

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn debug_is_redacted() {
        let s = Secret::new(String::from("hunter2"));
        assert_eq!(format!("{s:?}"), "Secret(<redacted>)");
        assert!(!format!("{s:?}").contains("hunter2"));
    }

    #[test]
    fn debug_inside_a_struct_is_also_redacted() {
        #[derive(Debug)]
        #[allow(dead_code)]
        struct Profile {
            name: String,
            key: Secret<Vec<u8>>,
        }
        let p = Profile {
            name: "wg0".into(),
            key: Secret::new(vec![1, 2, 3, 4]),
        };
        let rendered = format!("{p:?}");
        assert!(rendered.contains("wg0"));
        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains('1') || !rendered.contains("[1, 2, 3, 4]"));
    }

    #[test]
    fn expose_returns_the_value() {
        let s = Secret::new(String::from("secret-value"));
        assert_eq!(s.expose(), "secret-value");
    }

    #[test]
    fn expose_mut_allows_in_place_use() {
        let mut s = Secret::new(vec![1u8, 2, 3]);
        s.expose_mut().push(4);
        assert_eq!(s.expose(), &vec![1, 2, 3, 4]);
    }

    #[test]
    fn clone_preserves_contents() {
        let s = Secret::new(String::from("copy me"));
        let c = s.clone();
        assert_eq!(s.expose(), c.expose());
    }
}
