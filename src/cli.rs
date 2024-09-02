use std::{fmt::Display, fs, io, path::PathBuf};

use crate::config::{Color, DrawStrategy, Palette};

// Errors
#[derive(Debug)]
pub enum CliError {
    Io(io::Error),
    InvalidArg(String),
    InvalidValue(String),
    InvalidColor(String),
    NoSuchArg(String),
    NoArgValue(String),
    NonZeroSpeed,
}
// No, i dont want to use thiserror
impl Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::InvalidArg(a) => write!(f, "Invalid argument \"{a}\""),
            Self::InvalidValue(v) => write!(f, "Invalid argument value \"{v}\""),
            Self::InvalidColor(c) => write!(f, "Invalid color {c}"),
            Self::NoSuchArg(a) => write!(f, "No such argument \"{a}\""),
            Self::NoArgValue(a) => write!(f, "Expected a value for \"{a}\""),
            Self::NonZeroSpeed => write!(f, "Speed must be > 0"),
        }
    }
}

pub fn print_version() {
    println!("PITCH1002 v{}", env!("CARGO_PKG_VERSION"));
}
pub fn print_help() {
    print_version();
    // FIXME: i feel like the help message a bit messy
    println!("PITCH1002 v{} - {}", env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_DESCRIPTION"));
    println!("{}", env!("CARGO_PKG_AUTHORS"));
    println!();
    println!("USAGE:");
    println!("    pitch1002 <GAME.ch8> [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --palettes, -p <PALETTES>   Specify custom palette list separated by semicolons (see EXAMPLES)");
    println!("    --speed, -s <SPEED>         How many cycles will make CPU in one frame (20 is default)");
    println!("    --mute                      Mute audio on start");
    println!("    --draw-on-step              Redraw the screen on every CPU cycle, instead of once in a frame");
    println!("    --help, -h                  Print this message!");
    println!("    --version, -v               Print version");
    println!("    --hello                     Say \"hello\"");
    println!();
    println!("CONTROLS:");
    println!("         QWERTY                  CHIP─8     ");
    println!("    ┌───┬───┬───┬───┐      ┌───┬───┬───┬───┐");
    println!("    │ 1 │ 2 │ 3 │ 4 │      │ 1 │ 2 │ 3 │ C │");
    println!("    ├───┼───┼───┼───┤      ├───┼───┼───┼───┤");
    println!("    │ q │ W │ e │ r │      │ 4 │ 5 │ 6 │ D │");
    println!("    ├───┼───┼───┼───┤  ->  ├───┼───┼───┼───┤");
    println!("    │ A │ S │ D │ f │      │ 7 │ 8 │ 9 │ E │");
    println!("    ├───┼───┼───┼───┤      ├───┼───┼───┼───┤");
    println!("    │ z │ x │ c │ v │      │ A │ 0 │ B │ F │");
    println!("    └───┴───┴───┴───┘      └───┴───┴───┴───┘");
    println!();
    println!("    ┌─────┐");
    println!("    │ ESC │       - Pause/unpause the game");
    println!("    ├───┬─┘");
    println!("    │ M │         - Mute/unmute");
    println!("    ├───┼───┐");
    println!("    │ [ │ ] │     - Previous/next palette");
    println!("    ├───┼───┼───┐");
    println!("    │ 0 │ - │ + │ - Reset/-/+ speed");
    println!("    ├───┴───┴───┤");
    println!("    │   SPACE   │ - Fast forward!");
    println!("    ├───────────┤");
    println!("    │   ENTER   │ (during the pause) - Restart the game");
    println!("    └───────────┘");
    println!();
    println!("EXAMPLES:");
    println!("    Launch PITCH1002 and scan current dir for .ch8 files");
    println!("        pitch1002");
    println!("        pitch1002 first-game.ch8 second-game.ch8");
    println!("        pitch1002 other-dir/");
    println!("        pitch1002 dir-recursive/**/*");
    println!();
    println!("    Use custom palette (#foreground,#background)");
    println!("        pitch1002 ./game.ch8 --palettes #fff,#000");
    println!();
    println!("    Multiple palettes, to change them in-game!");
    println!("        pitch1002 ./game.ch8 --palettes #fff,#000;#e0f8d0,#081820;#f00,#111");
    println!();
    println!("    Enabling --draw-on-step may fix sprites disapearing");
    println!("    This may happen when sprite redraws too often and screen just have no time to update");
    println!("    Can have a big impact on performance if game speed is too high!");
    println!("        pitch1002 ./oh-no.ch8 --draw-on-step");
}

/// Cli
#[derive(Default)]
pub struct Cli {
    pub game_paths: Option<Vec<PathBuf>>,
    pub palettes: Option<Vec<Palette>>,
    pub speed: Option<u16>,
    pub mute: bool,
    pub draw_strategy: DrawStrategy,
}
impl Cli {
    pub fn new() -> Result<Self, CliError> {
        let mut args = std::env::args().skip(1);

        let mut cli = Self {
            game_paths: None,
            palettes: None,
            speed: None,
            mute: false,
            draw_strategy: DrawStrategy::default()
        };

        // Parse args
        while let Some(arg) = args.next() {
            match arg.as_str() {

                "help" | "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                "version" | "--version" | "-b" => {
                    print_version();
                    std::process::exit(0);
                }
                "--hello" => {
                    println!("hi!");
                    std::process::exit(0);
                }

                "--palettes" | "-p" => {
                    let mut pals = vec![];
                    let val = args.next()
                        .ok_or(CliError::NoArgValue(arg.clone()))?;

                    // Parse value similar to "#RRGGBB,#RRGGBB;..."
                    for palette in val.split(';') {
                        let (fg_str, bg_str) = palette
                            .split_once(',')
                            .ok_or(CliError::InvalidArg(arg.clone()))?;

                        let palette = (
                            Color::from_hex_str(fg_str)
                                .ok_or(CliError::InvalidColor(fg_str.into()))?,
                            Color::from_hex_str(bg_str)
                                .ok_or(CliError::InvalidColor(bg_str.into()))?
                        );

                        pals.push(palette);
                    }

                    cli.palettes = Some(pals);
                }

                "--speed" | "-s" => {
                    let val = args.next()
                        .ok_or(CliError::NoArgValue(arg.clone()))?;
                    let num = val
                        .parse::<u16>()
                        .map_err(|_| CliError::InvalidValue(val))?;

                    if num == 0 {
                        return Err(CliError::NonZeroSpeed);
                    }

                    cli.speed = Some(num);
                }

                "--mute" => {
                    cli.mute = true;
                }

                "--draw-on-step" => {
                    cli.draw_strategy = DrawStrategy::Step;
                }

                arg if arg.starts_with('-') => return Err(CliError::NoSuchArg(arg.into())),

                arg => {
                    let path: PathBuf = arg.into();
                    let mut paths = vec![];

                    if path.is_file() {
                        paths.push(path);
                    } else if path.is_dir() {
                        let dir = fs::read_dir(path)
                            .map_err(CliError::Io)?;

                        // Loop through dir entries
                        for entry in dir {
                            let entry = entry
                                .map_err(CliError::Io)?;
                            let entry_path = entry.path();

                            // Allow only files ending with .ch8
                            let is_chip = entry_path.extension().is_some_and(|e| e == "ch8");
                            if entry_path.is_file() && is_chip {
                                paths.push(entry_path);
                            }
                        }
                    }

                    cli.game_paths = Some(paths);
                }
            }
        }


        Ok(cli)
    }
}
