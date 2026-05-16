# obsidian-watch

Watch an Obsidian (Markdown) notes directory and run a configurable shell
command whenever notes are created, modified, or deleted.

## Status

Early but functional. The binary watches the configured directories
recursively, debounces bursty filesystem activity over a fixed 15-second
window, and runs the configured shell command per coalesced event with
`{FILE}` / `{PATH}` / `{OLD_FILE}` / `{OLD_PATH}` / `{EVENT}` /
`{TIMESTAMP}` substitution. The long debounce is deliberate — this tool is
not intended to be realtime. See [`SPEC.md`](SPEC.md) for the full design
and platform caveats.

## Build

```sh
cargo build              # debug build → target/debug/notify-obsidian
cargo build --release    # optimized build → target/release/notify-obsidian
```

## Install

```sh
cargo install --path .
```

## Usage

```sh
notify-obsidian -c config.toml
```

### Example config

```toml
[directories.notes]
path    = "/home/you/notes"
command = "/home/you/bin/event-handler.sh {FILE} {PATH} {EVENT} {TIMESTAMP}"
```

Each `[directories.<name>]` block names a watched directory:

- `path` — absolute path to the directory to watch (recursively).
- `command` — shell command to run on each event. The following tokens are
  substituted at invocation time:

| Token         | Value                                                  |
|---------------|--------------------------------------------------------|
| `{FILE}`      | The file's basename                                    |
| `{PATH}`      | The file's absolute path                               |
| `{OLD_FILE}`  | Previous basename on `rename`; empty otherwise         |
| `{OLD_PATH}`  | Previous absolute path on `rename`; empty otherwise    |
| `{EVENT}`     | `create`, `modify`, `delete`, or `rename`              |
| `{TIMESTAMP}` | Event time, RFC 3339 UTC                               |

For a rename, `{PATH}` / `{FILE}` are the **new** name and
`{OLD_PATH}` / `{OLD_FILE}` are the **old** name. Quote them in your
command (`--old-path "{OLD_PATH}"`) so non-rename events — where the old
tokens expand to empty strings — don't break argument parsing.

## Development

```sh
cargo test                       # run all tests
cargo test parses_minimal        # run a single test by name substring
cargo fmt                        # format
cargo clippy -- -D warnings      # lint (warnings as errors)
```

## Platform support

Built on the [`notify`](https://crates.io/crates/notify) crate, which uses
`inotify` on Linux, `FSEvents` on macOS, and `ReadDirectoryChangesW` on
Windows. Linux is the primary deployment target. Event behavior on macOS
differs in ways documented in `SPEC.md` — verify cross-platform behavior in
CI rather than trusting local testing.

## License

TBD.
