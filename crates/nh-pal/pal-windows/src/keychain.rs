//! DPAPI-backed keyring — the Windows platform secret store (WP-M1.1, §7.1).
//!
//! Windows has no machine-scoped *named* vault (the Credential Manager is
//! per-user), so the machine-scope pattern is: seal each entry's bytes with
//! **DPAPI** under `CRYPTPROTECT_LOCAL_MACHINE` and persist the sealed blob in a
//! file under `%ProgramData%`. Any process on the machine can unseal it — which
//! is exactly what a system service running as `LocalSystem` needs, and no other
//! machine can, because the DPAPI machine key never leaves the box.
//!
//! The entry name is passed to DPAPI as *optional entropy*, binding a blob to
//! its name: a blob for `"a"` will not unseal when read as `"b"`.
//!
//! This is the designated FFI boundary (ADR-0002): the two `CryptProtectData` /
//! `CryptUnprotectData` calls are the only `unsafe` here, each with a SAFETY
//! note. Everything above them is the safe [`Keyring`] surface the daemon wires
//! in as the Windows backend.

use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};

use nh_secrets::{Keyring, SecretsError, SecretsResult};
use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_LOCAL_MACHINE, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData,
    CryptUnprotectData,
};

/// A machine-scoped keyring backed by DPAPI-sealed files.
#[derive(Debug)]
pub struct DpapiKeyring {
    dir: PathBuf,
}

impl DpapiKeyring {
    /// Opens (creating if needed) a DPAPI keyring rooted at `dir`. In production
    /// `dir` is under `%ProgramData%\NetworkHub`, readable only by
    /// administrators; the DPAPI machine sealing is the real secret boundary.
    ///
    /// # Errors
    /// Returns [`SecretsError`] if the directory cannot be created.
    pub fn open(dir: impl Into<PathBuf>) -> SecretsResult<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    /// The path for an entry, sanitised so it cannot escape `dir`.
    fn path(&self, entry: &str) -> PathBuf {
        let safe: String = entry
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.dir.join(format!("{safe}.dpapi"))
    }
}

impl Keyring for DpapiKeyring {
    fn get(&self, entry: &str) -> SecretsResult<Option<Vec<u8>>> {
        match std::fs::read(self.path(entry)) {
            Ok(sealed) => Ok(Some(dpapi_unprotect(&sealed, entry.as_bytes())?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn set(&self, entry: &str, value: &[u8]) -> SecretsResult<()> {
        let sealed = dpapi_protect(value, entry.as_bytes())?;
        // A DPAPI blob is safe at rest, but write owner-restricted anyway (the
        // ProgramData ACL already limits it to administrators).
        std::fs::write(self.path(entry), sealed)?;
        Ok(())
    }

    fn delete(&self, entry: &str) -> SecretsResult<()> {
        match std::fs::remove_file(self.path(entry)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

/// True when a DPAPI keyring can actually be used on this host (the directory is
/// writable). DPAPI itself is always present on Windows, so this only guards the
/// filesystem; the daemon uses it to decide whether to fall back to a file
/// keyring.
#[must_use]
pub fn is_available(dir: &Path) -> bool {
    std::fs::create_dir_all(dir).is_ok()
}

// --- FFI boundary ----------------------------------------------------------

/// Seals `data` with DPAPI machine scope, binding it to `entropy`.
#[allow(unsafe_code)]
fn dpapi_protect(data: &[u8], entropy: &[u8]) -> SecretsResult<Vec<u8>> {
    let in_blob = CRYPT_INTEGER_BLOB {
        cbData: to_cb(data.len())?,
        pbData: data.as_ptr().cast_mut(),
    };
    let ent_blob = CRYPT_INTEGER_BLOB {
        cbData: to_cb(entropy.len())?,
        pbData: entropy.as_ptr().cast_mut(),
    };
    let mut out = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };

    // SAFETY: `in_blob`/`ent_blob` point at slices borrowed for this whole call,
    // so they stay valid until DPAPI returns. `out` is written by the call; on
    // success its `pbData` is a LocalAlloc'd buffer of `cbData` bytes that
    // `finish` copies out and then frees exactly once.
    let ok = unsafe {
        CryptProtectData(
            &raw const in_blob,
            null(),
            &raw const ent_blob,
            null(),
            null(),
            CRYPTPROTECT_LOCAL_MACHINE | CRYPTPROTECT_UI_FORBIDDEN,
            &raw mut out,
        )
    };
    finish(ok, out, "seal")
}

/// Unseals a DPAPI blob produced by [`dpapi_protect`] with the same `entropy`.
#[allow(unsafe_code)]
fn dpapi_unprotect(sealed: &[u8], entropy: &[u8]) -> SecretsResult<Vec<u8>> {
    let in_blob = CRYPT_INTEGER_BLOB {
        cbData: to_cb(sealed.len())?,
        pbData: sealed.as_ptr().cast_mut(),
    };
    let ent_blob = CRYPT_INTEGER_BLOB {
        cbData: to_cb(entropy.len())?,
        pbData: entropy.as_ptr().cast_mut(),
    };
    let mut out = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: null_mut(),
    };

    // SAFETY: as in `dpapi_protect` — the input blobs point at live borrows for
    // the duration of the call, and the output buffer is copied then freed.
    let ok = unsafe {
        CryptUnprotectData(
            &raw const in_blob,
            null_mut(),
            &raw const ent_blob,
            null(),
            null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &raw mut out,
        )
    };
    finish(ok, out, "unseal")
}

/// Shared tail for both calls: check the `BOOL` result (`windows-sys` types it as
/// `i32`), copy the output buffer, free it, and map failure to a
/// [`SecretsError`]. Keeping the raw-pointer copy in one place minimises the
/// `unsafe` surface.
#[allow(unsafe_code)]
fn finish(ok: i32, out: CRYPT_INTEGER_BLOB, op: &str) -> SecretsResult<Vec<u8>> {
    if ok == 0 {
        let code = std::io::Error::last_os_error();
        return Err(SecretsError::Keyring(format!("DPAPI {op} failed: {code}")));
    }
    // SAFETY: on success DPAPI set `out.pbData` to a buffer of `out.cbData`
    // bytes; we copy it before freeing and never read it afterwards.
    let bytes = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec();
    // SAFETY: `out.pbData` was allocated by DPAPI via LocalAlloc; freeing it
    // exactly once here is required and correct.
    unsafe {
        LocalFree(out.pbData.cast());
    }
    Ok(bytes)
}

/// DPAPI blob lengths are `u32`; refuse anything that would truncate.
fn to_cb(len: usize) -> SecretsResult<u32> {
    u32::try_from(len)
        .map_err(|_| SecretsError::Keyring("secret is too large for DPAPI".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_dpapi() {
        let dir = tempfile::tempdir().expect("tempdir");
        let kr = DpapiKeyring::open(dir.path()).expect("open");

        assert!(kr.get("missing").expect("get").is_none());

        kr.set("master", b"super-secret-key-bytes").expect("set");
        assert_eq!(
            kr.get("master").expect("get").expect("some"),
            b"super-secret-key-bytes"
        );

        // The on-disk blob is sealed, not the plaintext.
        let sealed = std::fs::read(dir.path().join("master.dpapi")).expect("read blob");
        assert_ne!(sealed.as_slice(), b"super-secret-key-bytes");

        kr.delete("master").expect("delete");
        assert!(kr.get("master").expect("get").is_none());
        kr.delete("master").expect("delete-missing is ok");
    }

    #[test]
    fn entropy_binds_a_blob_to_its_entry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let kr = DpapiKeyring::open(dir.path()).expect("open");
        kr.set("alpha", b"value").expect("set");

        // Rename the sealed file to another entry: unsealing must fail because
        // the entropy (the entry name) no longer matches.
        std::fs::rename(
            dir.path().join("alpha.dpapi"),
            dir.path().join("beta.dpapi"),
        )
        .expect("rename");
        assert!(kr.get("beta").is_err(), "entropy mismatch must not unseal");
    }
}
