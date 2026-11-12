use rune::{
    AppSpec, Color, Cx, El, KeyEvent, Key, TextStyle, Tx, Shadow,
    col, el, row, text,
};
use crate::state::AppState;

// Text styles
const TITLE_STYLE: TextStyle = TextStyle::new(16.0, 600.0);
const LABEL_STYLE: TextStyle = TextStyle::new(13.0, 500.0);
const VALUE_STYLE: TextStyle = TextStyle::new(12.0, 450.0);
const SMALL_STYLE: TextStyle = TextStyle::new(11.0, 400.0);

#[derive(Clone)]
pub struct UiState {
    pub sensitivity_x: f32,
    pub sensitivity_y: f32,
    pub deadzone: f32,
    pub trigger_button: String,
    pub grab_audio_enabled: bool,
    pub release_audio_enabled: bool,
    pub system_tray_enabled: bool,
    pub device_path: Option<String>,
}

impl Default for UiState {
    fn default() -> Self {
        UiState {
            sensitivity_x: 1.0,
            sensitivity_y: 1.0,
            deadzone: 0.05,
            trigger_button: "BTN_LEFT".to_string(),
            grab_audio_enabled: true,
            release_audio_enabled: true,
            system_tray_enabled: false,
            device_path: None,
        }
    }
}

impl From<AppState> for UiState {
    fn from(state: AppState) -> Self {
        UiState {
            sensitivity_x: state.sensitivity_x,
            sensitivity_y: state.sensitivity_y,
            deadzone: state.deadzone,
            trigger_button: state.trigger_button,
            grab_audio_enabled: state.audio_enabled,
            release_audio_enabled: state.audio_enabled,
            system_tray_enabled: state.minimize_to_tray,
            device_path: state.device_path,
        }
    }
}

// Global holder for initial state (since init must be fn())
static INITIAL_STATE: std::sync::OnceLock<UiState> = std::sync::OnceLock::new();

pub fn run_ui(initial_state: AppState) -> anyhow::Result<()> {
    log::info!("Starting mouse2joy UI...");

    // Convert to UI state and store in global for the init fn
    let ui_state: UiState = initial_state.into();
    let _ = INITIAL_STATE.set(ui_state.clone());

    let spec = AppSpec::<UiState> {
        scenes: &[],
        on_key: Some(on_key),
        poll: Some(poll_shared_state),
        ..AppSpec::new("mouse2joy", (1024.0, 600.0), ui_init, view)
    };

    rune::run(spec);

    log::info!("UI completed");
    Ok(())
}

fn ui_init() -> UiState {
    INITIAL_STATE.get().cloned().unwrap_or_default()
}

fn poll_shared_state(_state: &mut UiState, now: f64, _tx: &mut Tx) -> bool {
    // Keep the UI running
    now > 0.0
}

fn on_key(_state: &mut UiState, event: &KeyEvent, _tx: &mut Tx) -> bool {
    match &event.key {
        Key::Escape => {
            // Could trigger app quit here
            false
        }
        _ => false,
    }
}

fn view(state: &UiState, _cx: &mut Cx<UiState>) -> El {
    // Dark theme colors
    let bg = Color { r: 0.05, g: 0.05, b: 0.05, a: 1.0 };
    let panel_bg = Color { r: 0.1, g: 0.1, b: 0.1, a: 1.0 };
    let panel_border = Color { r: 0.18, g: 0.18, b: 0.22, a: 1.0 };
    let accent = Color { r: 0.35, g: 0.55, b: 1.0, a: 1.0 };
    let text_primary = Color { r: 0.92, g: 0.93, b: 0.95, a: 1.0 };
    let text_secondary = Color { r: 0.6, g: 0.6, b: 0.65, a: 1.0 };
    let slider_track = Color { r: 0.15, g: 0.15, b: 0.2, a: 1.0 };
    let slider_knob = Color::WHITE;
    let toggle_on = accent;
    let toggle_off = Color { r: 0.3, g: 0.3, b: 0.35, a: 1.0 };

    // Joystick indicator canvas (top-left)
    let joystick_canvas = create_joystick_canvas(panel_bg, panel_border, accent, text_secondary);

    // Device selector
    let device_selector = create_device_selector(&state.device_path, panel_bg, panel_border, text_primary, text_secondary);

    // Sensitivity sliders
    let x_sensitivity = create_slider(
        "X Sensitivity", state.sensitivity_x,
        text_primary, text_secondary, accent, slider_track, slider_knob,
        |v| format!("{:.0}%", v * 100.0)
    );
    let y_sensitivity = create_slider(
        "Y Sensitivity", state.sensitivity_y,
        text_primary, text_secondary, accent, slider_track, slider_knob,
        |v| format!("{:.0}%", v * 100.0)
    );
    let deadzone = create_slider(
        "Deadzone", state.deadzone,
        text_primary, text_secondary, accent, slider_track, slider_knob,
        |v| format!("{:.0}%", v * 100.0)
    );

    // Trigger button picker
    let trigger_button = create_trigger_button(&state.trigger_button, panel_bg, panel_border, text_primary, accent, text_secondary);

    // Toggles
    let grab_audio_toggle = create_toggle(
        "Grab Audio Cue", state.grab_audio_enabled,
        text_primary, text_secondary, toggle_on, toggle_off, panel_bg, panel_border
    );
    let release_audio_toggle = create_toggle(
        "Release Audio Cue", state.release_audio_enabled,
        text_primary, text_secondary, toggle_on, toggle_off, panel_bg, panel_border
    );
    let system_tray_toggle = create_toggle(
        "System Tray", state.system_tray_enabled,
        text_primary, text_secondary, toggle_on, toggle_off, panel_bg, panel_border
    );

    // Import/Export buttons
    let import_btn = create_button("Import Config", text_primary);
    let export_btn = create_button("Export Config", text_primary);

    // Main layout
    col()
        .w_full()
        .h_full()
        .fill(bg)
        .padding(20.0)
        .gap(16.0)
        .child(
            // Top row: joystick canvas + right panel (sliders)
            row()
                .w_full()
                .grow(1.0)
                .gap(16.0)
                .child(
                    // Left: joystick indicator
                    col()
                        .size(200.0, 200.0)
                        .gap(12.0)
                        .child(joystick_canvas)
                        .child(device_selector)
                )
                .child(
                    // Right: sliders panel
                    col()
                        .grow(1.0)
                        .gap(12.0)
                        .padding(16.0)
                        .radius(8.0)
                        .fill(panel_bg)
                        .border(1.0, panel_border)
                        .child(text("Sensitivity Settings").font(TITLE_STYLE).color(text_primary))
                        .child(x_sensitivity)
                        .child(y_sensitivity)
                        .child(deadzone)
                        .child(el().h(16.0))
                        .child(text("Trigger Button").font(TITLE_STYLE).color(text_primary))
                        .child(trigger_button)
                )
        )
        .child(
            // Bottom row: toggles + import/export
            row()
                .w_full()
                .gap(16.0)
                .child(
                    col()
                        .grow(1.0)
                        .gap(12.0)
                        .padding(16.0)
                        .radius(8.0)
                        .fill(panel_bg)
                        .border(1.0, panel_border)
                        .child(text("Options").font(TITLE_STYLE).color(text_primary))
                        .child(grab_audio_toggle)
                        .child(release_audio_toggle)
                        .child(system_tray_toggle)
                )
                .child(
                    col()
                        .size(200.0, 0.0)
                        .gap(12.0)
                        .padding(16.0)
                        .radius(8.0)
                        .fill(panel_bg)
                        .border(1.0, panel_border)
                        .child(text("Config").font(TITLE_STYLE).color(text_primary))
                        .child(import_btn)
                        .child(export_btn)
                )
        )
}

fn create_joystick_canvas(
    panel_bg: Color,
    panel_border: Color,
    accent: Color,
    text_secondary: Color,
) -> El {
    // Center position for joystick
    let center_x = 100.0;
    let center_y = 100.0;
    let stick_radius = 60.0;
    let knob_radius = 20.0;

    el()
        .key("joystick_canvas")
        .size(200.0, 200.0)
        .radius(8.0)
        .clip_rounded(8.0)
        .fill(panel_bg)
        .border(1.0, panel_border)
        .child(
            // Outer ring
            el()
                .absolute(center_x - stick_radius, center_y - stick_radius)
                .size(stick_radius * 2.0, stick_radius * 2.0)
                .radius(stick_radius)
                .border(2.0, panel_border)
        )
        .child(
            // Crosshair lines
            el()
                .absolute(center_x - stick_radius, center_y - 1.0)
                .size(stick_radius * 2.0, 2.0)
                .fill(panel_border)
        )
        .child(
            el()
                .absolute(center_x - 1.0, center_y - stick_radius)
                .size(2.0, stick_radius * 2.0)
                .fill(panel_border)
        )
        .child(
            // Knob (centered for now - would animate with mouse input)
            el()
                .absolute(center_x - knob_radius, center_y - knob_radius)
                .size(knob_radius * 2.0, knob_radius * 2.0)
                .radius(knob_radius)
                .fill(accent)
        )
        .child(
            text("Joystick").font(SMALL_STYLE).color(text_secondary).absolute(10.0, 10.0)
        )
}

fn create_device_selector(
    device_path: &Option<String>,
    panel_bg: Color,
    panel_border: Color,
    text_primary: Color,
    text_secondary: Color,
) -> El {
    let display_text = device_path
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("No device selected");

    el()
        .key("device_selector")
        .w_full()
        .h(40.0)
        .radius(6.0)
        .fill(panel_bg)
        .border(1.0, panel_border)
        .center_y()
        .px(12.0)
        .child(text(display_text).font(LABEL_STYLE).color(text_primary))
        .child(el().grow(1.0))
        .child(text("▼").font(SMALL_STYLE).color(text_secondary))
}

fn create_slider<F>(
    label: &str,
    value: f32,
    text_primary: Color,
    text_secondary: Color,
    accent: Color,
    slider_track: Color,
    slider_knob: Color,
    format_fn: F,
) -> El
where
    F: Fn(f32) -> String,
{
    let slider_width = 168.0;
    let slider_height = 20.0;
    let knob_diameter = 16.0;
    let bar_height = 4.0;
    let travel = slider_width - knob_diameter;
    let knob_x = knob_diameter / 2.0 + value * travel;
    let fill_width = knob_diameter / 2.0 + value * travel;

    row()
        .w_full()
        .h(50.0)
        .gap(12.0)
        .center_y()
        .child(text(label).font(LABEL_STYLE).color(text_primary).w(120.0))
        .child(
            el()
                .key("slider_track")
                .role("slider")
                .size(slider_width, slider_height)
                .child(
                    el()
                        .key("track_bg")
                        .size(slider_width, bar_height)
                        .absolute(0.0, (slider_height - bar_height) / 2.0)
                        .radius(bar_height / 2.0)
                        .fill(slider_track)
                )
                .child(
                    el()
                        .key("track_fill")
                        .size(fill_width, bar_height)
                        .absolute(0.0, (slider_height - bar_height) / 2.0)
                        .radius(bar_height / 2.0)
                        .fill(accent)
                )
                .child(
                    el()
                        .key("slider_knob")
                        .size(knob_diameter, knob_diameter)
                        .absolute(knob_x, (slider_height - knob_diameter) / 2.0)
                        .radius(knob_diameter / 2.0)
                        .fill(slider_knob)
                        .box_shadow(Shadow::core_animation(0.0, 1.0, 3.0, Color { r: 0.0, g: 0.0, b: 0.0, a: 0.3 }))
                )
        )
        .child(text(format_fn(value)).font(VALUE_STYLE).color(text_secondary).w(50.0))
}

fn create_trigger_button(
    trigger: &str,
    panel_bg: Color,
    panel_border: Color,
    text_primary: Color,
    accent: Color,
    text_secondary: Color,
) -> El {
    el()
        .key("trigger_button")
        .w_full()
        .h(40.0)
        .radius(6.0)
        .fill(panel_bg)
        .border(1.0, panel_border)
        .center_y()
        .px(12.0)
        .child(text("Trigger: ").font(LABEL_STYLE).color(text_primary))
        .child(text(trigger).font(LABEL_STYLE).color(accent))
        .child(el().grow(1.0))
        .child(text("Press to set").font(SMALL_STYLE).color(text_secondary))
}

fn create_toggle(
    label: &str,
    enabled: bool,
    text_primary: Color,
    text_secondary: Color,
    toggle_on: Color,
    toggle_off: Color,
    _panel_bg: Color,
    _panel_border: Color,
) -> El {
    let (track_color, thumb_x) = if enabled {
        (toggle_on, 28.0)
    } else {
        (toggle_off, 4.0)
    };

    row()
        .w_full()
        .h(40.0)
        .gap(12.0)
        .center_y()
        .child(text(label).font(LABEL_STYLE).color(text_primary).grow(1.0))
        .child(
            el()
                .size(48.0, 28.0)
                .radius(14.0)
                .fill(track_color)
                .child(
                    el()
                        .absolute(thumb_x, 2.0)
                        .size(24.0, 24.0)
                        .radius(12.0)
                        .fill(Color::WHITE)
                        .box_shadow(Shadow::core_animation(0.0, 1.0, 3.0, Color { r: 0.0, g: 0.0, b: 0.0, a: 0.3 }))
                )
        )
        .child(text(if enabled { "ON" } else { "OFF" }).font(VALUE_STYLE).color(if enabled { toggle_on } else { text_secondary }))
}

fn create_button(
    label: &str,
    text_primary: Color,
) -> El {
    let panel_bg = Color { r: 0.1, g: 0.1, b: 0.1, a: 1.0 };
    let panel_border = Color { r: 0.18, g: 0.18, b: 0.22, a: 1.0 };
    row()
        .w_full()
        .h(36.0)
        .center()
        .radius(6.0)
        .fill(panel_bg)
        .border(1.0, panel_border)
        .child(text(label).font(LABEL_STYLE).color(text_primary))
}