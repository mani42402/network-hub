# Desktop application (Flutter)

Unprivileged gRPC client of `networkhubd`. Lands in **WP-M0.12**.

Renders from the daemon's capability matrix (ADR-0006) — no platform conditionals in
feature code. Develop against `daemon-sim` (WP-M0.11) so the UI needs no root.
