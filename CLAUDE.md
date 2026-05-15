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


<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:7510c1e2 -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

**Architecture in one line:** issues live in a local Dolt DB; sync uses `refs/dolt/data` on your git remote; `.beads/issues.jsonl` is a passive export. See https://github.com/gastownhall/beads/blob/main/docs/SYNC_CONCEPTS.md for details and anti-patterns.

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->
