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
| create        | `Create(_)` (any subkind), `Modify(Name(RenameMode::To))`       |
| modify        | `Modify(Data(_))`                                               |
| delete        | `Remove(_)`, `Modify(Name(RenameMode::From))`                   |
| rename        | `Modify(Name(RenameMode::Both))`                                |

`Modify(Metadata(_))` (permissions, timestamps) is **ignored** — Obsidian
generates these during normal operation and they don't represent note edits.

A `rename` event fires once with `{PATH}` / `{FILE}` set to the new name
and `{OLD_PATH}` / `{OLD_FILE}` set to the old name. The rationale for one
event (rather than splitting into `delete` of the old path plus `create` of
the new path) is that a downstream consumer can't otherwise distinguish a
rename from an unrelated delete-and-create pair within the debounce window;
the `rename` verb is itself information.

### Platform behavior caveats

- **Rename detection.** Linux `inotify` produces paired
  `MOVED_FROM` / `MOVED_TO` events with a cookie, which `notify` surfaces as
  `RenameMode::Both` with `paths = [from, to]` — this is the case that
  produces a `rename` event. macOS `FSEvents` typically surfaces a rename as
  separate `Remove` + `Create`, which we emit as `delete` and `create`
  (information about the rename pairing is unavailable from the backend).
  When the backend only delivers one half of a rename (`RenameMode::From`
  without a matching `To`, or vice versa) we fall back to `delete` /
  `create` respectively.
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
| `{OLD_FILE}`  | Previous basename on `rename` events; empty string otherwise |
| `{OLD_PATH}`  | Previous absolute path on `rename` events; empty otherwise   |
| `{EVENT}`     | `create`, `modify`, `delete`, or `rename`                    |
| `{TIMESTAMP}` | Event time, RFC 3339 UTC (e.g. `2026-05-15T13:42:07Z`)       |

For non-rename events `{OLD_FILE}` and `{OLD_PATH}` expand to the empty
string. Quote them in your command (`--old-path "{OLD_PATH}"`) so the
shell receives an empty argument rather than dropping the flag's value.

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
