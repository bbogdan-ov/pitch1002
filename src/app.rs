mod context;
mod ui;

use std::{sync::Arc, time::{Duration, Instant}};

use context::Context;
use ui::Ui;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ ElementState, KeyEvent, StartCause, WindowEvent },
    event_loop::{ ActiveEventLoop, ControlFlow },
    keyboard::{ KeyCode, PhysicalKey },
    window::{ Window, WindowId },
};

use crate::{
    buzzer::Buzzer,
    config::{ Config, DrawStrategy, DEFAULT_SPEED, MAX_SPEED },
    cpu::{ Cpu, DISPLAY_DATA_LEN, DISPLAY_HEIGHT, DISPLAY_WIDTH },
};

// Consts
/// CHIP-8 display size * display scale = PITCH1002 window size
pub const DISPLAY_SCALE: u32 = 8;
/// Delay in milliseconds between frames (1000 / FPS)
pub const TARGET_DELAY: u64 = 16;

/// Convert [KeyCode] to CHIP-8 button
fn key_to_btn(keycode: KeyCode) -> Option<u8> {
    match keycode {
        KeyCode::Digit1 => Some(0x1),
        KeyCode::Digit2 => Some(0x2),
        KeyCode::Digit3 => Some(0x3),
        KeyCode::Digit4 => Some(0xC),

        KeyCode::KeyQ => Some(0x4),
        KeyCode::KeyW => Some(0x5),
        KeyCode::KeyE => Some(0x6),
        KeyCode::KeyR => Some(0xD),

        KeyCode::KeyA => Some(0x7),
        KeyCode::KeyS => Some(0x8),
        KeyCode::KeyD => Some(0x9),
        KeyCode::KeyF => Some(0xE),

        KeyCode::KeyZ => Some(0xA),
        KeyCode::KeyX => Some(0x0),
        KeyCode::KeyC => Some(0xB),
        KeyCode::KeyV => Some(0xF),
        _ => None
    }
}

/// App
pub struct App<'win> {
    win: Option<Arc<Window>>,
    context: Option<Context<'win>>,

    config: Config,
    cpu: Cpu,
    buzzer: Option<Buzzer>,
    ui: Ui,
    /// This screen data is NOT controlled by a CHIP-8 program,
    /// but only used to add my own UI "above" the CHIP-8 display
    screen: [Option<bool>; DISPLAY_DATA_LEN],

    is_paused: bool,
    is_fastforward: bool,

    last_time: Instant,
}
impl<'win> App<'win> {
    pub fn new(config: Config, cpu: Cpu, buzzer: Option<Buzzer>) -> Self {
        Self {
            win: None,
            context: None,

            config,
            cpu,
            buzzer,
            ui: Ui::new(),
            screen: [None; DISPLAY_DATA_LEN],

            is_paused: false,
            is_fastforward: false,

            last_time: Instant::now()
        }
    }

    fn handle_key(&mut self, keycode: KeyCode, pressed: bool) {
        if pressed {
            match keycode {
                // Next palette
                KeyCode::BracketRight => self.config.next_palette(),
                // Prev palette
                KeyCode::BracketLeft => self.config.prev_palette(),

                // Reset speed
                KeyCode::Digit0 => self.set_speed(DEFAULT_SPEED),
                // Increase speed
                KeyCode::Equal | KeyCode::NumpadAdd => self.increase_speed(),
                // Decrease speed
                KeyCode::Minus | KeyCode::NumpadSubtract => self.decrease_speed(),
                // Toggle mute
                KeyCode::KeyM => self.buzzer_toggle_mute(),

                // Toggle pause
                KeyCode::Escape => self.is_paused ^= true,
                // Enable fast forward
                KeyCode::Space => self.is_fastforward = true,

                // Restart the game and unpause (during the pause)
                KeyCode::Enter if self.is_paused => {
                    self.cpu.restart();
                    self.is_paused = false;
                },

                _ => ()
            }
        } else {
            match keycode {
                // Disable fast forward
                KeyCode::Space => self.is_fastforward = false,
                _ => ()
            }
        }

        // Change pressed button only if correct button was pressed
        let Some(code) = key_to_btn(keycode) else {
            return;
        };

        if pressed {
            self.cpu.button_pressed(code);
        } else {
            self.cpu.button_released(code);
        }
    }

    // Speed
    pub fn set_speed(&mut self, speed: u16) {
        self.config.speed = speed.clamp(1, MAX_SPEED);
        self.ui.speed_msg_timer = 30;
    }
    pub fn increase_speed(&mut self) {
        self.set_speed(self.config.speed + 1);
    }
    pub fn decrease_speed(&mut self) {
        self.set_speed(self.config.speed.saturating_sub(1));
    }

    // Buzzer
    pub fn buzzer_set_playing(&mut self, state: bool) {
        if let Some(buz) = &mut self.buzzer {
            buz.set_playing(state);
        }
    }
    pub fn buzzer_toggle_mute(&mut self) {
        if let Some(buz) = &mut self.buzzer {
            buz.set_muted(!buz.muted);
        }
    }

    fn render_screen(&mut self) {
        let ctx = self.context.as_mut().unwrap();

        // Copy screen data to render buffer
        for i in 0..DISPLAY_DATA_LEN {
            let pixel = self.screen[i].unwrap_or(self.cpu.display[i]);

            // RGB color
            let color =
                if pixel { self.config.fg() }
                else { self.config.bg() };

            ctx.buffer_data[i*4 + 0] = color.0; // Red
            ctx.buffer_data[i*4 + 1] = color.1; // Green
            ctx.buffer_data[i*4 + 2] = color.2; // Blue
            ctx.buffer_data[i*4 + 3] = 255; // Alpha
        }

        // Render the screen
        ctx.write_buf();
        ctx.render();
    }
}
impl<'win> ApplicationHandler for App<'win> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create a window
        let size = LogicalSize::new(DISPLAY_WIDTH * DISPLAY_SCALE, DISPLAY_HEIGHT * DISPLAY_SCALE);
        let attrs = Window::default_attributes()
            .with_title("PITCH1002")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_max_inner_size(size)
            .with_resizable(false);

        #[cfg(target_os = "linux")]
        let attrs = {
            use winit::platform::wayland::WindowAttributesExtWayland;

            attrs.with_name("pitch1002", "pitch1002")
        };

        let win = Arc::new(event_loop.create_window(attrs).unwrap());
        let mut context = Context::new(Arc::clone(&win));

        // First time render
        context.render();

        self.win = Some(win);
        self.context = Some(context);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                let elapsed = self.last_time.elapsed();
                let elapsed_ms = elapsed.as_millis() as u64;

                // Update only if actual FPS is lower than needed
                if elapsed_ms >= TARGET_DELAY {
                    self.last_time = Instant::now();

                    self.draw_ui();

                    if self.is_paused {
                        // Simply render the screen if paused
                        self.render_screen();
                        self.buzzer_set_playing(false);
                    } else {
                        let speed = 
                            if self.is_fastforward { 2 }
                            else { 1 };

                        for _ in 0..speed {
                            // Step cpu only if unpaused
                            for _ in 0..self.config.speed {
                                self.cpu.step();

                                // Step draw strategy
                                if self.config.draw_strategy == DrawStrategy::Step {
                                    self.render_screen();
                                }
                            }

                            // Update the timers
                            self.cpu.step_timers();
                        }

                        if self.cpu.st > 0 {
                            self.buzzer_set_playing(true);
                        } else {
                            self.buzzer_set_playing(false);
                        }

                        // Frame draw strategy
                        if self.config.draw_strategy == DrawStrategy::Frame {
                            self.render_screen();
                        }
                    }
                }

                // Set a delay between redraw requests
                let wait_ms = TARGET_DELAY.saturating_sub(elapsed_ms);
                let wait = Instant::now() + Duration::from_millis(wait_ms);
                event_loop.set_control_flow(ControlFlow::WaitUntil(wait));
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent { physical_key, state, .. },
                ..
            } => if let PhysicalKey::Code(keycode) = physical_key {
                self.handle_key(keycode, state == ElementState::Pressed)
            }
            WindowEvent::Resized(size) => {
                // Window resized
                self.context.as_mut().unwrap().resize(size);
            }
            WindowEvent::CloseRequested => {
                // Window closed
                event_loop.exit();
            }
            _ => ()
        }
    }
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::ResumeTimeReached { .. }) {
            // Request a redraw after delay
            self.win.as_ref().unwrap().request_redraw();
        }
    }
}
