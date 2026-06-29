mod backup;
mod restore;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "timeshine")]
#[command(about = "TimeShine: Multi-architecture backup tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Backup {
        #[arg(short, long, value_name = "DIR")]
        dir: PathBuf,
        #[arg(short, long)]
        system: bool,
    },
    Restore {
        #[arg(short, long, value_name = "SNAPSHOT_FILE")]
        snapshot: PathBuf,
        #[arg(short, long, value_name = "DEST_DIR")]
        dest: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Backup { dir, system } => {
            if let Err(e) = backup::run_backup(dir, *system) {
                eprintln!("Backup process failed: {}", e);
            }
        }
        Commands::Restore { snapshot, dest, dry_run } => {
            if let Err(e) = restore::run_restore(snapshot, dest, *dry_run) {
                eprintln!("Restore process failed: {}", e);
            }
        }
    }
}
