use clap::{Args, Parser, Subcommand};
use mindswall_core::{
    config::Config,
    ipc::{self, Command as Control, Response},
    random_image, scanner,
    scheduler::Scheduler,
    slideshow::Slideshow,
    wallpaper::{SystemWallpaper, WallpaperSetter},
};
use std::{
    path::PathBuf,
    process::ExitCode,
    sync::mpsc,
    time::{Duration, Instant},
};

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
fn run_slideshow() -> Result<(), Box<dyn std::error::Error>> {
    let dirs = directories::ProjectDirs::from("", "", "MindsWall")
        .ok_or("user data directory unavailable")?;
    std::fs::create_dir_all(dirs.data_dir())?;
    let appender = tracing_appender::rolling::never(dirs.data_dir(), "mindswall.log");
    let (writer, _guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(writer)
        .try_init()
        .map_err(|e| e.to_string())?;
    let path = Config::default_path()?;
    let config = Config::load(&path)?;
    let mut show = Slideshow::new(config.images()?, config.shuffle)?;
    let (tx, rx) = mpsc::channel();
    ipc::listen(tx).map_err(|e| {
        format!("cannot start slideshow control listener (another instance may be running): {e}")
    })?;
    let mut timer = Scheduler::new(Duration::from_secs(config.interval_seconds), Instant::now());
    SystemWallpaper.set_wallpaper(show.next())?;
    println!("Slideshow running. Press Ctrl+C to stop.");
    loop {
        let event = match timer.wait(Instant::now()) {
            Some(wait) => rx.recv_timeout(wait),
            None => rx.recv().map_err(|_| mpsc::RecvTimeoutError::Disconnected),
        };
        match event {
            Ok((command, reply)) => {
                let now = Instant::now();
                let result: Result<String, String> = match command {
                    Control::Pause => {
                        timer.pause(now);
                        Ok("paused".into())
                    }
                    Control::Resume => {
                        timer.resume(now);
                        Ok("running".into())
                    }
                    Control::Status => Ok(format!(
                        "{}: {}",
                        if timer.is_paused() {
                            "paused"
                        } else {
                            "running"
                        },
                        show.current()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default()
                    )),
                    Control::Next => {
                        let mut candidate = show.clone();
                        let path = candidate.next();
                        let result = SystemWallpaper
                            .set_wallpaper(path)
                            .map(|()| path.display().to_string())
                            .map_err(|e| e.to_string());
                        if result.is_ok() {
                            show = candidate;
                        }
                        timer.reset(now);
                        result
                    }
                    Control::Previous => {
                        let mut candidate = show.clone();
                        match candidate.previous() {
                            Some(path) => {
                                let result = SystemWallpaper
                                    .set_wallpaper(path)
                                    .map(|()| path.display().to_string())
                                    .map_err(|e| e.to_string());
                                if result.is_ok() {
                                    show = candidate;
                                }
                                timer.reset(now);
                                result
                            }
                            None => Err("no previous wallpaper".into()),
                        }
                    }
                };
                let response = match result {
                    Ok(message) => Response { ok: true, message },
                    Err(message) => {
                        tracing::warn!("{message}");
                        Response { ok: false, message }
                    }
                };
                let _ = reply.send(response);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let mut candidate = show.clone();
                match SystemWallpaper.set_wallpaper(candidate.next()) {
                    Ok(()) => show = candidate,
                    Err(error) => tracing::warn!(%error, "wallpaper update failed"),
                };
                timer.reset(Instant::now());
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("control listener stopped".into());
            }
        }
    }
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
