use clap::{Args, Parser, Subcommand};
use mindswall_core::{
    random_image, scanner,
    wallpaper::{SystemWallpaper, WallpaperSetter},
};
use std::{path::PathBuf, process::ExitCode};

#[derive(Parser)]
#[command(version, about = "Manage Windows desktop wallpapers")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Scan(Folders),
    Random(Folders),
    Set { file: PathBuf },
}
#[derive(Args)]
struct Folders {
    #[arg(required = true)]
    folders: Vec<PathBuf>,
    #[arg(long, conflicts_with = "no_recursive")]
    recursive: bool,
    #[arg(long, conflicts_with = "recursive")]
    no_recursive: bool,
}
fn run() -> Result<(), mindswall_core::Error> {
    match Cli::parse().command {
        Command::Scan(args) => {
            for path in scanner::scan(&args.folders, !args.no_recursive)? {
                println!("{}", path.display());
            }
        }
        Command::Random(args) => {
            let images = scanner::scan(&args.folders, !args.no_recursive)?;
            let image = random_image(&images)?;
            SystemWallpaper.set_wallpaper(image)?;
            println!("{}", image.display());
        }
        Command::Set { file } => SystemWallpaper.set_wallpaper(&file)?,
    }
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
