use crate::state::AppState;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use uinput::event::{Absolute, Controller, Event, absolute::Position, controller::GamePad};
use uinput::{device::Builder, Device as UinputDevice};

/// Virtual joystick emulator using uinput.
/// Creates a Linux joystick device and emits axis events based on mouse deltas.
pub struct JoystickEmulator {
    device: UinputDevice,
    // Joystick axis range (typically -32767 to 32767 for 16-bit)
    axis_min: i32,
    axis_max: i32,
    // Current axis positions
    x_pos: i32,
    y_pos: i32,
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
            x_pos: 0,
            y_pos: 0,
        })
    }

    /// Update joystick axes based on mouse deltas from AppState.
    /// This should be called in a loop at a regular interval (e.g., 60Hz).
    pub async fn update(&mut self, state: Arc<Mutex<AppState>>) {
        let (dx, dy, grabbed, sensitivity_x, sensitivity_y) = {
            let mut s = state.lock().await;
            let dx = s.mouse_dx;
            let dy = s.mouse_dy;
            // Reset deltas after reading
            s.mouse_dx = 0;
            s.mouse_dy = 0;
            (dx, dy, s.grabbed, s.sensitivity_x, s.sensitivity_y)
        };

        if !grabbed {
            // Return to center when not grabbed
            if self.x_pos != 0 || self.y_pos != 0 {
                self.x_pos = 0;
                self.y_pos = 0;
                let _ = self.device.send(Event::Absolute(Absolute::Position(Position::X)), 0);
                let _ = self.device.send(Event::Absolute(Absolute::Position(Position::Y)), 0);
                let _ = self.device.synchronize();
            }
            return;
        }

        // Apply deltas to position (clamped to axis range)
        let scale = 100.0; // Adjust this for sensitivity feel
        self.x_pos = (self.x_pos as f32 + dx as f32 * sensitivity_x * scale).clamp(self.axis_min as f32, self.axis_max as f32) as i32;
        self.y_pos = (self.y_pos as f32 + dy as f32 * sensitivity_y * scale).clamp(self.axis_min as f32, self.axis_max as f32) as i32;

        // Emit axis events
        let _ = self.device.send(Event::Absolute(Absolute::Position(Position::X)), self.x_pos);
        let _ = self.device.send(Event::Absolute(Absolute::Position(Position::Y)), self.y_pos);
        let _ = self.device.synchronize();
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
        self.x_pos = 0;
        self.y_pos = 0;
        self.device.send(Event::Absolute(Absolute::Position(Position::X)), 0)?;
        self.device.send(Event::Absolute(Absolute::Position(Position::Y)), 0)?;
        self.device.synchronize()?;
        Ok(())
    }
}

/// Run the joystick emulator loop
pub async fn run_joystick_emulator(state: Arc<Mutex<AppState>>) -> anyhow::Result<()> {
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
                // Audio will be played by the audio module
            }
        } else if !grabbed && last_grabbed {
            // Just released - release trigger button and center
            let _ = emulator.release_trigger();
            let _ = emulator.center();
            if release_audio {
                // Audio will be played by the audio module
            }
        }
        last_grabbed = grabbed;

        // Update axes
        emulator.update(state.clone()).await;

        // Run at ~60Hz
        sleep(Duration::from_millis(16)).await;
    }
}