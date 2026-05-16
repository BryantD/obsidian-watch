use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use notify::EventKind;
use notify::event::{Event, ModifyKind, RenameMode};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

#[derive(Debug)]
pub struct EventContext<'a> {
    pub file: &'a str,
    pub path: &'a str,
    pub old_file: &'a str,
    pub old_path: &'a str,
    pub event: &'a str,
    pub timestamp: &'a str,
}

#[derive(Debug)]
pub struct Classification<'a> {
    pub path: &'a Path,
    pub old_path: Option<&'a Path>,
    pub event: &'static str,
}

pub fn substitute(template: &str, ctx: &EventContext) -> String {
    template
        .replace("{FILE}", ctx.file)
        .replace("{PATH}", ctx.path)
        .replace("{OLD_FILE}", ctx.old_file)
        .replace("{OLD_PATH}", ctx.old_path)
        .replace("{EVENT}", ctx.event)
        .replace("{TIMESTAMP}", ctx.timestamp)
}

pub fn classify_event(event: &Event) -> Vec<Classification<'_>> {
    match &event.kind {
        EventKind::Create(_) => event
            .paths
            .iter()
            .map(|p| Classification {
                path: p.as_path(),
                old_path: None,
                event: "create",
            })
            .collect(),
        EventKind::Remove(_) => event
            .paths
            .iter()
            .map(|p| Classification {
                path: p.as_path(),
                old_path: None,
                event: "delete",
            })
            .collect(),
        EventKind::Modify(ModifyKind::Metadata(_)) => Vec::new(),
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => event
            .paths
            .iter()
            .map(|p| Classification {
                path: p.as_path(),
                old_path: None,
                event: "delete",
            })
            .collect(),
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => event
            .paths
            .iter()
            .map(|p| Classification {
                path: p.as_path(),
                old_path: None,
                event: "create",
            })
            .collect(),
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
            // notify pairs RenameMode::Both as [from, to]. Anything else is malformed;
            // fall back to a single 'modify' on each path so we don't drop the event.
            if event.paths.len() == 2 {
                vec![Classification {
                    path: event.paths[1].as_path(),
                    old_path: Some(event.paths[0].as_path()),
                    event: "rename",
                }]
            } else {
                event
                    .paths
                    .iter()
                    .map(|p| Classification {
                        path: p.as_path(),
                        old_path: None,
                        event: "modify",
                    })
                    .collect()
            }
        }
        EventKind::Modify(_) => event
            .paths
            .iter()
            .map(|p| Classification {
                path: p.as_path(),
                old_path: None,
                event: "modify",
            })
            .collect(),
        _ => Vec::new(),
    }
}

pub fn render_and_spawn(template: &str, ctx: &EventContext) -> Result<Child> {
    let rendered = substitute(template, ctx);
    Command::new("sh")
        .arg("-c")
        .arg(&rendered)
        .spawn()
        .with_context(|| format!("spawning shell command: {rendered}"))
}

pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn basename(path: &Path) -> &str {
    path.file_name().and_then(|n| n.to_str()).unwrap_or("")
}

pub fn find_command<'a>(path: &Path, dirs: &'a [(PathBuf, String)]) -> Option<&'a str> {
    dirs.iter()
        .find(|(dir, _)| path.starts_with(dir))
        .map(|(_, cmd)| cmd.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{
        CreateKind, DataChange, EventAttributes, MetadataKind, ModifyKind, RemoveKind, RenameMode,
    };

    fn ctx() -> EventContext<'static> {
        EventContext {
            file: "note.md",
            path: "/notes/note.md",
            old_file: "",
            old_path: "",
            event: "modify",
            timestamp: "2026-05-15T10:00:00Z",
        }
    }

    fn make_event(kind: EventKind, paths: Vec<&str>) -> Event {
        Event {
            kind,
            paths: paths.into_iter().map(PathBuf::from).collect(),
            attrs: EventAttributes::new(),
        }
    }

    #[test]
    fn substitutes_all_tokens() {
        let out = substitute("{EVENT} {FILE} at {PATH} ({TIMESTAMP})", &ctx());
        assert_eq!(
            out,
            "modify note.md at /notes/note.md (2026-05-15T10:00:00Z)"
        );
    }

    #[test]
    fn substitutes_old_tokens_for_rename() {
        let c = EventContext {
            file: "bar.md",
            path: "/notes/bar.md",
            old_file: "foo.md",
            old_path: "/notes/foo.md",
            event: "rename",
            timestamp: "2026-05-15T10:00:00Z",
        };
        let out = substitute("{EVENT}: {OLD_FILE} -> {FILE}", &c);
        assert_eq!(out, "rename: foo.md -> bar.md");
    }

    #[test]
    fn substitutes_old_path_empty_for_non_rename() {
        let out = substitute("{EVENT} {FILE} (was: {OLD_FILE})", &ctx());
        assert_eq!(out, "modify note.md (was: )");
    }

    #[test]
    fn leaves_unknown_braces_alone() {
        let out = substitute("{UNKNOWN} {FILE}", &ctx());
        assert_eq!(out, "{UNKNOWN} note.md");
    }

    #[test]
    fn handles_paths_with_spaces() {
        let c = EventContext {
            file: "my note.md",
            path: "/notes/my note.md",
            old_file: "",
            old_path: "",
            event: "create",
            timestamp: "2026-05-15T10:00:00Z",
        };
        let out = substitute("echo '{FILE}' '{PATH}'", &c);
        assert_eq!(out, "echo 'my note.md' '/notes/my note.md'");
    }

    #[test]
    fn classifies_create() {
        let e = make_event(EventKind::Create(CreateKind::File), vec!["/a/x.md"]);
        let out = classify_event(&e);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].path, Path::new("/a/x.md"));
        assert_eq!(out[0].event, "create");
        assert!(out[0].old_path.is_none());
    }

    #[test]
    fn classifies_remove_as_delete() {
        let e = make_event(EventKind::Remove(RemoveKind::File), vec!["/a/x.md"]);
        let out = classify_event(&e);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].event, "delete");
    }

    #[test]
    fn classifies_data_modify_as_modify() {
        let e = make_event(
            EventKind::Modify(ModifyKind::Data(DataChange::Content)),
            vec!["/a/x.md"],
        );
        let out = classify_event(&e);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].event, "modify");
        assert!(out[0].old_path.is_none());
    }

    #[test]
    fn classifies_rename_from_as_delete() {
        let e = make_event(
            EventKind::Modify(ModifyKind::Name(RenameMode::From)),
            vec!["/a/x.md"],
        );
        let out = classify_event(&e);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].path, Path::new("/a/x.md"));
        assert_eq!(out[0].event, "delete");
        assert!(out[0].old_path.is_none());
    }

    #[test]
    fn classifies_rename_to_as_create() {
        let e = make_event(
            EventKind::Modify(ModifyKind::Name(RenameMode::To)),
            vec!["/a/y.md"],
        );
        let out = classify_event(&e);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].path, Path::new("/a/y.md"));
        assert_eq!(out[0].event, "create");
        assert!(out[0].old_path.is_none());
    }

    #[test]
    fn classifies_rename_both_as_single_rename_event() {
        let e = make_event(
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
            vec!["/a/x.md", "/a/y.md"],
        );
        let out = classify_event(&e);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].path, Path::new("/a/y.md"));
        assert_eq!(out[0].old_path, Some(Path::new("/a/x.md")));
        assert_eq!(out[0].event, "rename");
    }

    #[test]
    fn classifies_rename_both_with_malformed_paths_falls_back_to_modify() {
        // notify is supposed to give us [from, to] for RenameMode::Both. If for any
        // reason we get a different shape, fall back to firing 'modify' rather than
        // panicking or dropping the event.
        let e = make_event(
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
            vec!["/a/x.md"],
        );
        let out = classify_event(&e);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].event, "modify");
    }

    #[test]
    fn ignores_metadata_modify() {
        let e = make_event(
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::Permissions)),
            vec!["/a/x.md"],
        );
        assert!(classify_event(&e).is_empty());
    }

    #[test]
    fn basename_extracts_filename() {
        assert_eq!(basename(Path::new("/foo/bar/baz.md")), "baz.md");
    }

    #[test]
    fn basename_handles_no_directory() {
        assert_eq!(basename(Path::new("file.md")), "file.md");
    }

    #[test]
    fn render_and_spawn_runs_substituted_shell_command() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let tmp_path = tmp.path().to_str().unwrap();
        let template =
            format!(r#"printf "%s,%s,%s" "{{FILE}}" "{{EVENT}}" "{{TIMESTAMP}}" > {tmp_path}"#);
        let c = EventContext {
            file: "alpha.md",
            path: "/n/alpha.md",
            old_file: "",
            old_path: "",
            event: "create",
            timestamp: "2026-05-15T10:00:00Z",
        };

        let mut child = render_and_spawn(&template, &c).expect("spawn");
        let status = child.wait().expect("wait");
        assert!(status.success(), "shell command exited non-zero");

        let written = std::fs::read_to_string(tmp.path()).unwrap();
        assert_eq!(written, "alpha.md,create,2026-05-15T10:00:00Z");
    }

    #[test]
    fn render_and_spawn_fails_for_unrunnable_shell() {
        let c = ctx();
        let result = render_and_spawn("exit 0", &c);
        assert!(result.is_ok());
    }

    #[test]
    fn find_command_matches_by_path_prefix() {
        let dirs = vec![
            (PathBuf::from("/a"), "cmd-a".to_string()),
            (PathBuf::from("/b/c"), "cmd-bc".to_string()),
        ];
        assert_eq!(find_command(Path::new("/a/file.md"), &dirs), Some("cmd-a"));
        assert_eq!(
            find_command(Path::new("/b/c/sub/file.md"), &dirs),
            Some("cmd-bc")
        );
        assert_eq!(find_command(Path::new("/z/file.md"), &dirs), None);
    }

    #[test]
    fn find_command_respects_component_boundaries() {
        // /ab does NOT start with /a as a path (component-wise).
        let dirs = vec![(PathBuf::from("/a"), "cmd-a".to_string())];
        assert_eq!(find_command(Path::new("/ab/file.md"), &dirs), None);
    }
}
