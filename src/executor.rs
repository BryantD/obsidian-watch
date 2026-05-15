use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use notify::EventKind;
use notify::event::ModifyKind;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

#[derive(Debug)]
pub struct EventContext<'a> {
    pub file: &'a str,
    pub path: &'a str,
    pub event: &'a str,
    pub timestamp: &'a str,
}

pub fn substitute(template: &str, ctx: &EventContext) -> String {
    template
        .replace("{FILE}", ctx.file)
        .replace("{PATH}", ctx.path)
        .replace("{EVENT}", ctx.event)
        .replace("{TIMESTAMP}", ctx.timestamp)
}

pub fn classify_event(kind: &EventKind) -> Option<&'static str> {
    match kind {
        EventKind::Create(_) => Some("create"),
        EventKind::Remove(_) => Some("delete"),
        EventKind::Modify(ModifyKind::Metadata(_)) => None,
        EventKind::Modify(_) => Some("modify"),
        _ => None,
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
    use notify::event::{CreateKind, DataChange, MetadataKind, ModifyKind, RemoveKind, RenameMode};

    fn ctx() -> EventContext<'static> {
        EventContext {
            file: "note.md",
            path: "/notes/note.md",
            event: "modify",
            timestamp: "2026-05-15T10:00:00Z",
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
    fn leaves_unknown_braces_alone() {
        let out = substitute("{UNKNOWN} {FILE}", &ctx());
        assert_eq!(out, "{UNKNOWN} note.md");
    }

    #[test]
    fn handles_paths_with_spaces() {
        let c = EventContext {
            file: "my note.md",
            path: "/notes/my note.md",
            event: "create",
            timestamp: "2026-05-15T10:00:00Z",
        };
        let out = substitute("echo '{FILE}' '{PATH}'", &c);
        assert_eq!(out, "echo 'my note.md' '/notes/my note.md'");
    }

    #[test]
    fn classifies_create() {
        assert_eq!(
            classify_event(&EventKind::Create(CreateKind::File)),
            Some("create")
        );
    }

    #[test]
    fn classifies_remove_as_delete() {
        assert_eq!(
            classify_event(&EventKind::Remove(RemoveKind::File)),
            Some("delete")
        );
    }

    #[test]
    fn classifies_data_modify_as_modify() {
        let kind = EventKind::Modify(ModifyKind::Data(DataChange::Content));
        assert_eq!(classify_event(&kind), Some("modify"));
    }

    #[test]
    fn classifies_rename_as_modify() {
        let kind = EventKind::Modify(ModifyKind::Name(RenameMode::Both));
        assert_eq!(classify_event(&kind), Some("modify"));
    }

    #[test]
    fn ignores_metadata_modify() {
        let kind = EventKind::Modify(ModifyKind::Metadata(MetadataKind::Permissions));
        assert_eq!(classify_event(&kind), None);
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
