# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project status

Bootstrapped. The binary parses a TOML config, opens a recursive `notify`
watcher per configured directory, and logs events to stderr. Command
execution and token substitution (`{FILE}` / `{PATH}` / `{EVENT}` /
`{TIMESTAMP}`) are **not yet implemented** — that is the next vertical slice
and is testable in isolation; do it red/green.

## Layout

- `src/main.rs` — CLI parsing (clap), wiring, and the event loop.
- `src/config.rs` — `Config` / `DirectoryConfig` structs and TOML loading
  (with unit tests).
- `SPEC.md` — design intent. Authoritative when it disagrees with `main.rs`
  (in practice, expect this when implementing features the binary doesn't do
  yet).

## Language

Rust, on purpose. This is an intentional deviation from the user's global
preference for Python (recorded in `SPEC.md` under "Decisions"). Do not
propose porting to Python or suggest a Python rewrite as an alternative.

## File-watching backend

Use the cross-platform [`notify`](https://crates.io/crates/notify) crate
(wraps `inotify` on Linux, FSEvents on macOS, `ReadDirectoryChangesW` on
Windows). **This supersedes the `inotify(7)`-specific masks listed in
`SPEC.md`** — the spec predates this decision and will be reconciled in a
follow-up. Treat events as the `notify` crate exposes them (create / modify /
remove / rename), not as raw inotify masks.

## Build, run, test, lint

- Build: `cargo build` (debug) / `cargo build --release`
- Run: `cargo run -- -c config.toml`
- Test: `cargo test` — single test: `cargo test <name>`
- Format: `cargo fmt` (check-only: `cargo fmt --check`)
- Lint: `cargo clippy -- -D warnings` (treat warnings as errors — required
  to pass before commit)

## Repo conventions (in addition to the global `~/.claude/CLAUDE.md`)

- Track work in **beads** (`bd`). Create issues for bootstrap and each
  vertical slice rather than working freehand.
- Maintain the **napkin** at `.claude/napkin.md` — read it before starting
  work, prune it after.
- `SPEC.md` is design intent; `README.md` is human-facing usage. Keep them
  in sync as features land. If you change behavior, update both in the same
  commit.
