use crate::state::AppState;
use std::sync::Arc;
use tokio::sync::Mutex;

use rune::{App, Widget, widgets::{Button, Checkbox, Dropdown, Slider, TextBox, Canvas}};
use rune_widgets::{Layout, Align};

/// Runs the Rune UI, taking a shared `AppState`.
pub async fn run_ui(state: Arc<Mutex<AppState>>) -> anyhow::Result<()> {
    // Create a new Rune application.
    let mut app = App::new();

    // Build UI layout – this is a very high‑level placeholder.
    let ui = Layout::new()
        .with_child(
            // Top‑left joystick indicator (placeholder canvas)
            Canvas::new(|_ctx| {
                // In a real implementation we would draw a circle representing the joystick.
            })
            .size((200, 200))
            .align(Align::TopLeft),
        )
        .with_child(
            // Right‑side sliders + textbox for sensitivities
            Layout::vertical()
                .with_child(
                    Slider::new("Sensitivity X", 0.1..=5.0)
                        .on_change(move |val| {
                            let st = state.clone();
                            tokio::spawn(async move {
                                let mut s = st.lock().await;
                                s.sensitivity_x = val as f32;
                            });
                        })
                )
                .with_child(
                    TextBox::new("", move |txt| {
                        if let Ok(v) = txt.parse::<f32>() {
                            let st = state.clone();
                            tokio::spawn(async move {
                                let mut s = st.lock().await;
                                s.sensitivity_x = v;
                            });
                        }
                    })
                )
                .align(Align::TopRight),
        )
        // Bottom‑left import/export and toggles – placeholders
        .with_child(
            Layout::horizontal()
                .with_child(Button::new("Import", move |_| {
                    // Open file dialog – Rune provides a dialog helper (not shown here).
                }))
                .with_child(Button::new("Export", move |_| {
                    // Export current config.
                }))
                .with_child(Checkbox::new("Minimize to tray", false, move |checked| {
                    let st = state.clone();
                    tokio::spawn(async move {
                        let mut s = st.lock().await;
                        s.minimize_to_tray = checked;
                    });
                }))
                .with_child(Checkbox::new("Audio cues", true, move |checked| {
                    let st = state.clone();
                    tokio::spawn(async move {
                        let mut s = st.lock().await;
                        s.audio_enabled = checked;
                    });
                }))
                .align(Align::BottomLeft),
        );

    app.set_root(ui);
    // Run the UI event loop. This blocks until the window is closed.
    app.run()?;
    Ok(())
}
