use crate::cli::Cli;

// Macros
#[macro_export]
macro_rules! color_hex {
    ($hex:expr) => {
        {
            let r = ($hex >> 16) as u8;
            let g = (($hex & 0xFF00) >> 8) as u8;
            let b = ($hex & 0xFF) as u8;

            crate::config::Color(r, g, b)
        }
    };
}
#[macro_export]
macro_rules! palette {
    ($fg:expr, $bg:expr) => {
        (crate::color_hex!($fg), crate::color_hex!($bg))
    };
}

// Types
pub type Palette = (Color, Color);

// Consts
/// FEEL FREE TO ADD YOUR OWN PALETTE!
/// (and please leave a author/link to where you got this palette from if this palette is not yours)
const DEFAULT_PALETTES: [Palette; 15] = [
    // My own palette :) Im proud of it
    palette!(0xdddddd, 0x000000),
    // https://lospec.com/palette-list/1-bit-error-4
    palette!(0xd2b7ff, 0x060010),
    // https://lospec.com/palette-list/1bit-monitor-glow
    palette!(0xf0f6f0, 0x222323),
    // https://lospec.com/palette-list/vanilla-milkshake
    palette!(0xd9c8bf, 0x28282e),
    // https://lospec.com/palette-list/dreamscape8
    palette!(0xc9cca1, 0x515262),
    // https://lospec.com/palette-list/cc-29
    palette!(0xb2b47e, 0x212123),
    // https://lospec.com/palette-list/18-bytes
    palette!(0xc8d0d8, 0x302828),
    // https://lospec.com/palette-list/chasm
    palette!(0x4593a5, 0x32313b),
    // https://lospec.com/palette-list/lcd-drab-4
    palette!(0xa9a77f, 0x1a1b00),
    // https://lospec.com/palette-list/ammo-8
    palette!(0xbedc7f, 0x112318),
    // https://lospec.com/palette-list/fantasy-24
    palette!(0xefd8a1, 0x2a1d0d),
    // https://lospec.com/palette-list/slso8
    palette!(0xffd4a3, 0x0d2b45),
    // https://lospec.com/palette-list/twilight-5
    palette!(0xee8695, 0x292831),
    // https://lospec.com/palette-list/kirokaze-gameboy
    palette!(0xe2f3e4, 0x332c50),
    // https://lospec.com/palette-list/blessing
    palette!(0xd8bfd8, 0x74569b),
];
pub const MAX_SPEED: u16 = 40000;
pub const DEFAULT_SPEED: u16 = 20;

/// Color
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Color(pub u8, pub u8, pub u8);
impl Color {
    pub fn from_hex_str(s: &str) -> Option<Color> {
        let hex = s.strip_prefix('#')?;
        if !hex.is_ascii() { return None }

        if hex.len() == 3 {
            // Parse #RGB
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;

            Some(Self(r, g, b))
        } else if hex.len() == 6 {
            // Parse #RRGGBB
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

            Some(Self(r, g, b))
        } else {
            None
        }
    }
}

/// When to draw the screen
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DrawStrategy {
    /// Draw the screen every frame (60 times per second)
    /// Suitable for most games
    #[default]
    Frame,
    /// Draw the screen on each draw/clear call (cpu cycle)
    /// May fix sprites "disapearing". Sometimes the sprites can be redrawn several times per
    /// frame, which is provokes the disapearance (the screen just doesn't have time to update)
    /// Can have a big impact on performance if game speed is too high!
    Step
}

/// Config
#[derive(Debug)]
pub struct Config {
    pub palettes: Vec<Palette>,
    /// (foreground, background)
    pub palette: Palette,
    pub cur_palette_index: usize,

    pub speed: u16,

    pub draw_strategy: DrawStrategy
}
impl Config {
    pub fn new(cli: Cli) -> Self {
        let palettes = cli.palettes.unwrap_or(DEFAULT_PALETTES.to_vec());

        Self {
            palette: palettes[0].clone(),
            palettes,
            cur_palette_index: 0,

            speed: cli.speed.unwrap_or(DEFAULT_SPEED),

            draw_strategy: cli.draw_strategy
        }
    }

    pub fn next_palette(&mut self) {
        let new_index = (self.cur_palette_index + 1) % (self.palettes.len() - 1);

        self.palette = self.palettes[new_index].clone();
        self.cur_palette_index = new_index;
    }
    pub fn prev_palette(&mut self) {
        let new_index =
            if self.cur_palette_index == 0 { self.palettes.len() - 1 }
            else { self.cur_palette_index - 1 };

        self.palette = self.palettes[new_index].clone();
        self.cur_palette_index = new_index;
    }

    pub fn fg(&self) -> &Color {
        &self.palette.0
    }
    pub fn bg(&self) -> &Color {
        &self.palette.1
    }
}
