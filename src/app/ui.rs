use crate::{cpu::{DISPLAY_DATA_LEN, DISPLAY_HEIGHT, DISPLAY_WIDTH}, font::UI_FONT};

use super::App;

/// Ui
pub struct Ui {
    pub speed_msg_timer: u8,
}
impl Ui {
    pub fn new() -> Self {
        Self {
            speed_msg_timer: 0
        }
    }
}

impl<'win> App<'win> {
    pub fn draw_ui(&mut self) {
        let dw = DISPLAY_WIDTH as u8;
        let dh = DISPLAY_HEIGHT as u8;

        // Clear ui screen
        self.screen.fill(None);

        // Draw speed message box
        if self.ui.speed_msg_timer > 0 {
            let w = dw;

            self.draw_rect(0, 0, w, 7, true);
            self.draw_rect(0, 7, w, 1, false);
            self.draw_text(&format!("speed {}", self.config.speed), 1, 1, false);

            self.ui.speed_msg_timer -= 1;
        }

        // Draw pause message box
        if self.is_paused {
            let w = dw;
            let h = 7;
            let x = 0;
            let y = dh - h;

            self.draw_rect(x, y-1, w, 1, false);
            self.draw_rect(x, y, w, h, true);
            self.draw_text("paused", x + 1, y + 1, false);
        }

        // Draw fast forward message box
        if self.is_fastforward {
            let w = 7;
            let h = 5;
            let x = DISPLAY_WIDTH as u8 - w - 1;
            let y = DISPLAY_HEIGHT as u8 - h - 1;
            self.draw_rect(x - 1, y - 1, w + 2, h + 2, false);
            self.draw_rect(x, y, w, h, true);
            // >>
            self.draw_sprite(
                &[
                    0b10010000,
                    0b11011000,
                    0b10010000,
                ],
                x + 1,
                y + 1,
                false,
            )
        }
    }

    /// Draw a filled rect on the screen
    fn draw_rect(&mut self, x: u8, y: u8, w: u8, h: u8, on: bool) {
        let x = x as usize;
        let y = y as usize;
        let w = w as usize;
        let h = h as usize;
        let sw = DISPLAY_WIDTH as usize;

        for row in 0..h {
            let line = y + row;
            let start = line * sw + x;
            let end = (line * sw + x + w).min(DISPLAY_DATA_LEN);

            self.screen[start..end].fill(Some(on));
        }
    }
    /// Draw a text on the screen
    /// Be a good boy, and use only lowercase characters
    fn draw_text(&mut self, text: &str, x: u8, y: u8, on: bool) {
        for (char_idx, chr) in text.chars().enumerate() {
            if chr == ' ' { continue }

            let ascii = chr as u8;

            let font_idx =
                if ascii >= 48 && ascii <= 57 { ascii - 48 + 1 } // 0-9
                else if ascii >= 97 && ascii <= 122 { ascii - 97 + 11 } // A-Z
                else if ascii == 33 { 37 } // !
                else if ascii == 63 { 38 } // ?
                else if ascii == 46 { 39 } // .
                else if ascii == 62 { 40 } // >
                else if ascii == 60 { 41 } // <
                else if ascii == 47 { 42 } // /
                else if ascii == 124 { 43 } // |
                else if ascii == 92 { 44 } // \
                else if ascii == 45 { 45 } // -
                else { 0 }; // Everything else

            let font_row = font_idx as usize * 5;
            let Some(sprite) = UI_FONT.get(font_row..font_row + 5) else {
                return;
            };

            self.draw_sprite(
                sprite,
                x + char_idx as u8 * 5,
                y,
                on,
            );
        }
    }
    fn draw_sprite(&mut self, rows: &[u8], x: u8, y: u8, on: bool) {
        let x = x as usize;
        let y = y as usize;
        let sw = DISPLAY_WIDTH as usize;
        let sh = DISPLAY_HEIGHT as usize;

        for row in 0..rows.len() {
            let mut pixels = rows[row];

            for col in 0..8 {
                if pixels & 0x80 != 0 {
                    let cx = (x + col) % sw;
                    let cy = (y + row) % sh;

                    let idx = cy * sw + cx;

                    self.screen[idx as usize] = Some(on);
                }

                pixels <<= 1;
            }
        }
    }
}
