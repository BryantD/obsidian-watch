mod config;
mod executor;

use anyhow::{Context, Result};
use clap::Parser;
use config::Config;
use notify::{Event, RecursiveMode};
use notify_debouncer_full::new_debouncer;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

const DEBOUNCE_WINDOW: Duration = Duration::from_secs(15);

#[derive(Parser)]
#[command(
    name = "notify-obsidian",
    about = "Watch an Obsidian notes directory and run a shell command on file events."
)]
struct Cli {
    #[arg(short, long)]
    config: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::from_path(&cli.config)?;

    let mut directories: Vec<(PathBuf, String)> = Vec::with_capacity(config.directories.len());
    for (name, dir) in &config.directories {
        let canonical = dir
            .path
            .canonicalize()
            .with_context(|| format!("resolving directory '{}' at {}", name, dir.path.display()))?;
        directories.push((canonical.clone(), dir.command.clone()));
    }

    let (tx, rx) = mpsc::channel();
    let mut debouncer = new_debouncer(DEBOUNCE_WINDOW, None, tx)?;

    for (canonical, command) in &directories {
        debouncer
            .watch(canonical, RecursiveMode::Recursive)
            .with_context(|| format!("watching {}", canonical.display()))?;
        eprintln!(
            "watching {} (command: {}, debounce: {:?})",
            canonical.display(),
            command,
            DEBOUNCE_WINDOW
        );
    }

    for result in rx {
        match result {
            Ok(events) => {
                for debounced in events {
                    handle_event(&debounced.event, &directories);
                }
            }
            Err(errors) => {
                for e in errors {
                    eprintln!("watch error: {e}");
                }
            }
        }
    }

    Ok(())
}

fn handle_event(event: &Event, dirs: &[(PathBuf, String)]) {
    let classifications = executor::classify_event(event);
    if classifications.is_empty() {
        return;
    }
    let timestamp = executor::now_rfc3339();

    for c in classifications {
        let Some(template) = executor::find_command(c.path, dirs) else {
            continue;
        };
        let path_str = c.path.display().to_string();
        let old_path_str = c
            .old_path
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let old_file = c.old_path.map(executor::basename).unwrap_or("");
        let ctx = executor::EventContext {
            file: executor::basename(c.path),
            path: &path_str,
            old_file,
            old_path: &old_path_str,
            event: c.event,
            timestamp: &timestamp,
        };
        match executor::render_and_spawn(template, &ctx) {
            Ok(mut child) => {
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
            }
            Err(e) => eprintln!("spawn failed: {e:#}"),
        }
    }
}
