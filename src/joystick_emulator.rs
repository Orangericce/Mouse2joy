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
    // Visual position for UI display
    // When NOT grabbed: tracks raw mouse movement (for device identification)
    // When GRABBED: tracks actual joystick position (x_pos/y_pos) with centering
    visual_x: f32,
    visual_y: f32,
    // Raw mouse accumulation for device identification when not grabbed
    raw_visual_x: f32,
    raw_visual_y: f32,
    // Track previous grab state to detect transitions
    was_grabbed: bool,
    // Track device path to detect device changes and reset state
    last_device_path: Option<String>,
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
            raw_visual_x: 0.0,
            raw_visual_y: 0.0,
            was_grabbed: false,
            last_device_path: None,
        })
    }

    /// Update joystick axes based on mouse deltas from AppState and update visual state.
    /// This should be called in a loop at a regular interval (e.g., 60Hz).
    /// Visual behavior:
    ///   - When NOT grabbed: shows raw mouse movement (for device identification)
    ///   - When GRABBED: shows actual joystick position with spring-back centering
    /// The virtual uinput device only emits axis values when grabbed.
    /// Also applies spring-back centering (matching Python reference).
    pub async fn update(&mut self, state: Arc<Mutex<AppState>>, visual: &VisualState) {
        let (dx, dy, grabbed, device_path, sensitivity_x, sensitivity_y, center_strength_x, center_strength_y, center_while_grabbed) = {
            let mut s = state.lock().await;
            let dx = s.mouse_dx;
            let dy = s.mouse_dy;
            // Reset deltas after reading
            s.mouse_dx = 0;
            s.mouse_dy = 0;
            (dx, dy, s.grabbed, s.device_path.clone(), s.sensitivity_x, s.sensitivity_y, s.center_strength_x, s.center_strength_y, s.center_while_grabbed)
        };

        // Check for device path change and reset state if needed
        if self.last_device_path != device_path {
            log::info!("Joystick emulator: device changed {:?} -> {:?}, resetting state", self.last_device_path, device_path);
            self.last_device_path = device_path;
            self.reset_position();
            // Also update visual to show centered state
            visual.set_knob(0.0, 0.0);
        }

        // Debug: log grab state changes
        if grabbed != self.was_grabbed {
            log::debug!("Joystick emulator: grabbed changed: {} -> {}", self.was_grabbed, grabbed);
        }

        let scale = 100.0;

        // Detect grab state transitions
        let _grab_pressed = grabbed && !self.was_grabbed;
        let grab_released = !grabbed && self.was_grabbed;
        self.was_grabbed = grabbed;

        // Always accumulate raw mouse movement for device identification
        // (when not grabbed, this is what the user sees)
        self.raw_visual_x += dx as f32 * sensitivity_x * scale;
        self.raw_visual_y += dy as f32 * sensitivity_y * scale;
        self.raw_visual_x = self.raw_visual_x.clamp(self.axis_min as f32, self.axis_max as f32);
        self.raw_visual_y = self.raw_visual_y.clamp(self.axis_min as f32, self.axis_max as f32);

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

        // Determine visual position based on grab state:
        // - Not grabbed: show raw mouse movement (device identification)
        // - Grabbed: show actual joystick position (with centering)
        let (display_x, display_y) = if grabbed {
            (self.x_pos, self.y_pos)
        } else {
            (self.raw_visual_x, self.raw_visual_y)
        };

        // On grab release, reset raw visual so it starts fresh next time
        if grab_released {
            self.raw_visual_x = 0.0;
            self.raw_visual_y = 0.0;
        }

        // Normalize to -1.0 to 1.0 for visualizer
        let visual_x = display_x / 32767.0;
        let visual_y = display_y / 32767.0;

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
        self.raw_visual_x = 0.0;
        self.raw_visual_y = 0.0;
        self.was_grabbed = false;
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
        self.raw_visual_x = 0.0;
        self.raw_visual_y = 0.0;
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