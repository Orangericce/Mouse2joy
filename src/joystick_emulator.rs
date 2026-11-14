use crate::audio::AudioPlayer;
use crate::state::AppState;
use crate::ui::VisualState;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use uinput::event::{Absolute, Controller, Event, absolute::Position, controller::GamePad};
use uinput::{device::Builder, Device as UinputDevice};

/// Virtual joystick emulator using uinput.
/// Creates a Linux joystick device and emits axis events based on mouse deltas.
/// Includes spring-back centering (like the Python reference).
pub struct JoystickEmulator {
    device: UinputDevice,
    // Joystick axis range (typically -32767 to 32767 for 16-bit)
    axis_min: i32,
    axis_max: i32,
    // Current axis positions (float for smooth centering) - used for uinput
    x_pos: f32,
    y_pos: f32,
    // Visual position - always tracks mouse movement for UI display, even when not grabbed
    visual_x: f32,
    visual_y: f32,
}

impl JoystickEmulator {
    /// Create a new virtual joystick device.
    /// Requires root or uinput group permissions.
    pub fn new() -> anyhow::Result<Self> {
        // Create uinput device with joystick capabilities
        let mut builder = Builder::default()?;
        builder = builder.name("mouse2joy Virtual Joystick")?;
        builder = builder.event(Event::Absolute(Absolute::Position(Position::X)))?;
        builder = builder.max(32767).min(-32767);
        builder = builder.event(Event::Absolute(Absolute::Position(Position::Y)))?;
        builder = builder.max(32767).min(-32767);
        builder = builder.event(Event::Absolute(Absolute::Position(Position::RX)))?;
        builder = builder.max(32767).min(-32767);
        builder = builder.event(Event::Absolute(Absolute::Position(Position::RY)))?;
        builder = builder.max(32767).min(-32767);
        builder = builder.event(Event::Controller(Controller::GamePad(GamePad::South)))?;
        builder = builder.event(Event::Controller(Controller::GamePad(GamePad::East)))?;
        builder = builder.event(Event::Controller(Controller::GamePad(GamePad::West)))?;
        builder = builder.event(Event::Controller(Controller::GamePad(GamePad::North)))?;
        let device = builder.create()?;

        log::info!("Created virtual joystick device: mouse2joy Virtual Joystick");

        Ok(JoystickEmulator {
            device,
            axis_min: -32767,
            axis_max: 32767,
            x_pos: 0.0,
            y_pos: 0.0,
            visual_x: 0.0,
            visual_y: 0.0,
        })
    }

    /// Update joystick axes based on mouse deltas from AppState and update visual state.
    /// This should be called in a loop at a regular interval (e.g., 60Hz).
    /// The visual knob position ALWAYS reflects mouse movement (even when not grabbed),
    /// so the user can see which device is active. The virtual uinput device only
    /// emits axis values when grabbed.
    /// Also applies spring-back centering (matching Python reference).
    pub async fn update(&mut self, state: Arc<Mutex<AppState>>, visual: &VisualState) {
        let (dx, dy, grabbed, sensitivity_x, sensitivity_y, center_strength_x, center_strength_y, center_while_grabbed) = {
            let mut s = state.lock().await;
            let dx = s.mouse_dx;
            let dy = s.mouse_dy;
            // Reset deltas after reading
            s.mouse_dx = 0;
            s.mouse_dy = 0;
            (dx, dy, s.grabbed, s.sensitivity_x, s.sensitivity_y, s.center_strength_x, s.center_strength_y, s.center_while_grabbed)
        };

        // Always apply mouse deltas to visual position (for UI display, even when not grabbed)
        // This lets the user see mouse movement to identify the correct device
        let scale = 100.0;
        self.visual_x += dx as f32 * sensitivity_x * scale;
        self.visual_y += dy as f32 * sensitivity_y * scale;
        // Clamp visual position to axis range
        self.visual_x = self.visual_x.clamp(self.axis_min as f32, self.axis_max as f32);
        self.visual_y = self.visual_y.clamp(self.axis_min as f32, self.axis_max as f32);

        // Spring-back centering for actual joystick position (uinput)
        // Runs when center_while_grabbed is true, or when not grabbed
        let should_center = center_while_grabbed || !grabbed;

        if should_center {
            // Apply spring-back: pull toward center by a fraction of the distance
            // Python: state["x"] -= state["x"] * CENTER_STRENGTH_X
            self.x_pos -= self.x_pos * center_strength_x;
            self.y_pos -= self.y_pos * center_strength_y;
        }

        // Apply mouse deltas to actual joystick position (only when grabbed, matching Python reference)
        if grabbed {
            self.x_pos += dx as f32 * sensitivity_x * scale;
            self.y_pos += dy as f32 * sensitivity_y * scale;
        }

        // Clamp actual position to axis range
        self.x_pos = self.x_pos.clamp(self.axis_min as f32, self.axis_max as f32);
        self.y_pos = self.y_pos.clamp(self.axis_min as f32, self.axis_max as f32);

        // Normalize visual position to -1.0 to 1.0 for visualizer
        let visual_x = self.visual_x / 32767.0;
        let visual_y = self.visual_y / 32767.0;

        // Always update visual state with visual position and grab status
        visual.set_knob(visual_x, visual_y);
        visual.set_grabbed(grabbed);

        let x_int = self.x_pos as i32;
        let y_int = self.y_pos as i32;

        if grabbed {
            // Emit axis events to the virtual uinput device
            let _ = self.device.send(Event::Absolute(Absolute::Position(Position::X)), x_int);
            let _ = self.device.send(Event::Absolute(Absolute::Position(Position::Y)), y_int);
            let _ = self.device.synchronize();
        } else {
            // Keep virtual device centered when not grabbed
            let _ = self.device.send(Event::Absolute(Absolute::Position(Position::X)), 0);
            let _ = self.device.send(Event::Absolute(Absolute::Position(Position::Y)), 0);
            let _ = self.device.synchronize();
        }
    }

    /// Reset the accumulated position to center. Called when transitioning
    /// from grabbed to ungrabbed, so the visual starts fresh next grab.
    pub fn reset_position(&mut self) {
        self.x_pos = 0.0;
        self.y_pos = 0.0;
        self.visual_x = 0.0;
        self.visual_y = 0.0;
    }

    /// Press the trigger button on the virtual joystick
    pub fn press_trigger(&mut self) -> anyhow::Result<()> {
        self.device.send(Event::Controller(Controller::GamePad(GamePad::South)), 1)?;
        self.device.synchronize()?;
        Ok(())
    }

    /// Release the trigger button on the virtual joystick
    pub fn release_trigger(&mut self) -> anyhow::Result<()> {
        self.device.send(Event::Controller(Controller::GamePad(GamePad::South)), 0)?;
        self.device.synchronize()?;
        Ok(())
    }

    /// Center the joystick axes
    pub fn center(&mut self) -> anyhow::Result<()> {
        self.x_pos = 0.0;
        self.y_pos = 0.0;
        self.visual_x = 0.0;
        self.visual_y = 0.0;
        self.device.send(Event::Absolute(Absolute::Position(Position::X)), 0)?;
        self.device.send(Event::Absolute(Absolute::Position(Position::Y)), 0)?;
        self.device.synchronize()?;
        Ok(())
    }
}

/// Run the joystick emulator loop
pub async fn run_joystick_emulator(state: Arc<Mutex<AppState>>, visual: &VisualState) -> anyhow::Result<()> {
    let mut emulator = JoystickEmulator::new()?;
    let mut last_grabbed = false;

    loop {
        // Check grab state for edge detection
        let (grabbed, grab_audio, release_audio) = {
            let s = state.lock().await;
            (s.grabbed, s.grab_audio_enabled, s.release_audio_enabled)
        };

        // Handle grab/release edge events
        if grabbed && !last_grabbed {
            // Just grabbed - press trigger button
            let _ = emulator.press_trigger();
            if grab_audio {
                AudioPlayer::new().play_grab();
            }
        } else if !grabbed && last_grabbed {
            // Just released - release trigger button and center virtual device
            let _ = emulator.release_trigger();
            let _ = emulator.center();
            if release_audio {
                AudioPlayer::new().play_release();
            }
        }
        last_grabbed = grabbed;

        // Update axes (includes spring-back centering)
        emulator.update(state.clone(), visual).await;

        // Run at ~60Hz (matches Python's CENTER_TICK_HZ = 60)
        sleep(Duration::from_millis(16)).await;
    }
}