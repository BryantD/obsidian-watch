# Purpose

This program watches an Obsidian notes directory (Markdown files) and triggers
a configurable shell command when files are created, modified, or deleted.

# Decisions

## Language: Rust

Deliberate deviation from the general "prefer Python" rule. Reasons:

- Fast startup and low steady-state memory (this runs as a long-lived daemon).
- Mature cross-platform file-watching ecosystem.
- Single static binary is easy to deploy on a NAS / home server.

## File-watching backend: the `notify` crate

The original sketch of this spec called for raw `inotify(7)` on Linux. That
was replaced with the cross-platform [`notify`] crate, which wraps:

- `inotify` on Linux
- `FSEvents` on macOS
- `ReadDirectoryChangesW` on Windows

[`notify`]: https://crates.io/crates/notify

We use the **recommended watcher** (`notify::recommended_watcher`) so the
backend is selected at compile time per target platform. The watcher is
configured `RecursiveMode::Recursive`, which delegates the new-subdirectory
race condition to the platform backend — meaning we no longer need the
"recreate the watcher every hour" workaround from the original spec.

### Events we care about

Expressed in `notify` crate vocabulary, not raw inotify masks:

| Logical event | `notify::EventKind` variants                                    |
|---------------|-----------------------------------------------------------------|
| create        | `Create(_)` (any subkind)                                       |
| modify        | `Modify(Data(_))` (content), `Modify(Name(_))` (rename)         |
| delete        | `Remove(_)`                                                     |

`Modify(Metadata(_))` (permissions, timestamps) is **ignored** — Obsidian
generates these during normal operation and they don't represent note edits.

### Platform behavior caveats

- **Rename detection.** Linux `inotify` produces paired
  `MOVED_FROM` / `MOVED_TO` events with a cookie; macOS `FSEvents` typically
  surfaces this as separate remove + create. The `notify` crate flattens this
  inconsistently. **In v1 we surface all rename events as `modify`** rather
  than splitting them into delete-of-old + create-of-new, because the latter
  requires per-path event classification and careful handling of
  `RenameMode::Both`. Splitting renames into delete + create is tracked as a
  follow-up bead.
- **Coalescing.** macOS coalesces rapid changes; Linux does not. Event
  ordering is best-effort, not guaranteed identical across platforms.
- **Editor save dance.** Obsidian (like many editors) saves by writing to a
  temp file and renaming over the target. Expect a `Create` of the temp
  file, a `Remove` of the original, and a `Create` of the final name. The
  configured command should be idempotent or tolerant of this pattern.

# CLI

```
notify-obsidian -c|--config <path-to-config.toml>
```

# Config

TOML file. Each `[directories.<name>]` block describes one watched directory
and the command to run on each event.

```toml
[directories.notes]
path    = "/home/durrell/notes"
command = "/home/durrell/bin/event-handler.sh {FILE} {PATH} {EVENT} {TIMESTAMP}"
```

Substitution tokens in `command`:

| Token         | Value                                                        |
|---------------|--------------------------------------------------------------|
| `{FILE}`      | The file's basename (e.g. `2026-05-15.md`)                   |
| `{PATH}`      | The file's absolute path                                     |
| `{EVENT}`     | `create`, `modify`, or `delete`                              |
| `{TIMESTAMP}` | Event time, RFC 3339 UTC (e.g. `2026-05-15T13:42:07Z`)       |

The `path` field is required so the config block name (`notes`) can be a
friendly label independent of the filesystem path.

# Debouncing

Events are coalesced through the `notify-debouncer-full` crate with a fixed
**15-second** quiet window. The window is deliberately long: this tool is
not realtime, and the typical use case (running a sync / index / backup
command after a note edit) tolerates seconds of latency in exchange for
collapsing a multi-event editor save into one command invocation. The
debouncer also drops "delete-then-create-same-path" sequences (interpreted
as a single modify-equivalent change).

The window is hardcoded; making it configurable is out of scope for now.

# Out of scope (for v1)

- A configurable debounce window (intentionally hardcoded at 15s).
- Filtering by glob or filename pattern (everything in the watched directory
  triggers the command).
- Concurrency limits on spawned commands.
- Daemonization (run under systemd / launchd / a terminal multiplexer).
