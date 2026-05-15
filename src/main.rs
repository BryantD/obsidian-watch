mod config;
mod executor;

use anyhow::{Context, Result};
use clap::Parser;
use config::Config;
use notify::{Event, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;

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
    let mut watcher = notify::recommended_watcher(tx)?;

    for (canonical, command) in &directories {
        watcher
            .watch(canonical, RecursiveMode::Recursive)
            .with_context(|| format!("watching {}", canonical.display()))?;
        eprintln!("watching {} (command: {})", canonical.display(), command);
    }

    for result in rx {
        match result {
            Ok(event) => handle_event(&event, &directories),
            Err(e) => eprintln!("watch error: {e}"),
        }
    }

    Ok(())
}

fn handle_event(event: &Event, dirs: &[(PathBuf, String)]) {
    let Some(event_kind) = executor::classify_event(&event.kind) else {
        return;
    };
    let timestamp = executor::now_rfc3339();

    for path in &event.paths {
        let Some(template) = executor::find_command(path, dirs) else {
            continue;
        };
        let path_str = path.display().to_string();
        let ctx = executor::EventContext {
            file: executor::basename(path),
            path: &path_str,
            event: event_kind,
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
