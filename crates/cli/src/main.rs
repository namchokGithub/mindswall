use clap::{Args, Parser, Subcommand};
use mindswall_core::{
    config::Config,
    ipc::{self, Command as Control},
    random_image, scanner,
    service::run_slideshow,
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
    Set {
        file: PathBuf,
    },
    Run,
    Next,
    Previous,
    Pause,
    Resume,
    Status,
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}
#[derive(Subcommand)]
enum ConfigCommand {
    Show,
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
fn run() -> Result<(), Box<dyn std::error::Error>> {
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
        Command::Config {
            command: ConfigCommand::Show,
        } => {
            let config = Config::load(&Config::default_path()?)?;
            println!("{}", serde_json::to_string_pretty(&config)?);
        }
        Command::Run => run_slideshow()?,
        command => {
            let control = match command {
                Command::Next => Control::Next,
                Command::Previous => Control::Previous,
                Command::Pause => Control::Pause,
                Command::Resume => Control::Resume,
                Command::Status => Control::Status,
                _ => unreachable!(),
            };
            let response = ipc::send(control)?;
            if !response.ok {
                return Err(response.message.into());
            }
            println!("{}", response.message);
        }
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
