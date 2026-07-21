# SPIKE-B — Windows service lifecycle, named-pipe SDDL, client identity

**Questions** (ARCHITECTURE.md §3.1, §10; ADR-0002):

1. Install / start / stop / uninstall a Windows service from a Rust binary, with **recovery
   actions** (auto-restart on crash).
2. Serve a **named pipe whose ACL** (via an SDDL string) admits only SYSTEM, Administrators,
   and a custom operators group — the ADR-0002 authorization boundary on Windows.
3. Identify the calling process with **`GetNamedPipeClientProcessId`** — the Windows analogue
   of `SO_PEERCRED`.
4. On start, **detect and clean orphaned state** from a previous run (the Windows F-2/F-4
   analogue).

## What is verified, and what is not

| Item | Status | Needs |
|---|---|---|
| Compiles for Windows (`x86_64-pc-windows-gnu`) | ✅ cross-checked from macOS | — |
| API usage: `windows-service` SCM, SDDL pipe, client-PID | ✅ type-checked | — |
| Service install/start/stop/uninstall at runtime | ⏳ scripted | a **Windows box**, elevated shell |
| Recovery actions (restart on crash) | ⏳ scripted | Windows |
| Pipe ACL admits the right principals only | ⏳ scripted | Windows |
| Client PID identification | ⏳ scripted | Windows |

The cross-compile validates the API surface — it caught four real errors during development
(a moved failure-actions method, a pipe constant relocated to `Storage::FileSystem`, and a
missing `Win32_System_IO` feature). Runtime behaviour needs a Windows machine.

**Caveat:** the cross-check uses the `windows-gnu` target; the product ships `windows-msvc`.
The two share the same API surface, so this catches API and type errors, but MSVC-specific
runtime behaviour (and the SCM interaction) is only confirmed by `scripts\run.ps1` on Windows.

## Running it (on Windows, elevated)

```powershell
powershell -ExecutionPolicy Bypass -File scripts\run.ps1
```

Ends with `SPIKE-B: PASS` only if install, start, probe, and clean uninstall all succeed.

## Design notes carried into the real build (pal-windows, WP-M0.7)

- **SDDL for the pipe.** `D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)` grants full control to
  SYSTEM and Administrators and read/write to Interactive Users. The real daemon replaces
  `IU` with the SID of a dedicated **NetworkHub Operators** group (ADR-0002); `IU` is used
  here only so the spike runs without first creating a group.
- **Client identity.** `GetNamedPipeClientProcessId` gives the caller's PID; from there the
  daemon opens the process token and checks group membership. The spike stops at the PID,
  which is enough to prove the mechanism.
- **Recovery + orphan check together** are the Windows fail-open story: the SCM restarts a
  crashed service, and the on-start `clean_orphans` reclaims stale state — the counterpart to
  SPIKE-D's boot-cleanup unit on Linux.
- **Two `unsafe` blocks** call into `windows-sys` for the SDDL conversion and pipe creation.
  In `pal-windows` these live behind the PAL boundary with a `SAFETY:` note per block, as the
  workspace lint policy requires.
