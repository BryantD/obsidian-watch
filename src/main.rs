mod config;

use anyhow::{Context, Result};
use clap::Parser;
use config::Config;
use notify::{RecursiveMode, Watcher};
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

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx)?;

    for (name, dir) in &config.directories {
        watcher
            .watch(&dir.path, RecursiveMode::Recursive)
            .with_context(|| format!("watching '{}' at {}", name, dir.path.display()))?;
        eprintln!(
            "watching '{}' at {} (command: {})",
            name,
            dir.path.display(),
            dir.command
        );
    }

    for result in rx {
        match result {
            Ok(event) => eprintln!("event: {event:?}"),
            Err(e) => eprintln!("watch error: {e}"),
        }
    }

    Ok(())
}
