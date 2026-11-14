// Lock-free shared visual state for joystick knob position (-1.0 to 1.0)
pub struct VisualState {
    pub knob_x: std::sync::atomic::AtomicU32,
    pub knob_y: std::sync::atomic::AtomicU32,
    pub grabbed: std::sync::atomic::AtomicBool,
}

impl VisualState {
    pub fn new() -> Self {
        VisualState {
            knob_x: std::sync::atomic::AtomicU32::new(0.0f32.to_bits()),
            knob_y: std::sync::atomic::AtomicU32::new(0.0f32.to_bits()),
            grabbed: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub fn set_knob(&self, x: f32, y: f32) {
        use std::sync::atomic::Ordering;
        self.knob_x.store(x.to_bits(), Ordering::Relaxed);
        self.knob_y.store(y.to_bits(), Ordering::Relaxed);
    }

    pub fn knob(&self) -> (f32, f32) {
        use std::sync::atomic::Ordering;
        (
            f32::from_bits(self.knob_x.load(Ordering::Relaxed)),
            f32::from_bits(self.knob_y.load(Ordering::Relaxed)),
        )
    }

    pub fn set_grabbed(&self, grabbed: bool) {
        use std::sync::atomic::Ordering;
        self.grabbed.store(grabbed, Ordering::Relaxed);
    }

    pub fn is_grabbed(&self) -> bool {
        use std::sync::atomic::Ordering;
        self.grabbed.load(Ordering::Relaxed)
    }
}

impl Default for VisualState {
    fn default() -> Self {
        Self::new()
    }
}

// Global handle to the shared visual state, so the UI poll (which cannot await
// the tokio mutex) can read the latest knob position without blocking.
pub static VISUAL: OnceLock<std::sync::Arc<VisualState>> = OnceLock::new();

/// Create (once) and return the shared visual state handle.
pub fn shared_visual() -> std::sync::Arc<VisualState> {
    VISUAL
        .get_or_init(|| std::sync::Arc::new(VisualState::new()))
        .clone()
}

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
    pub device_list: Vec<String>,
    pub trigger_listening: bool,
    pub device_dropdown_open: bool,
    // Spring-back centering (matches Python reference)
    pub center_strength_x: f32,
    pub center_strength_y: f32,
    pub center_while_grabbed: bool,
    // Runtime state from shared AppState
    pub grabbed: bool,
    pub mouse_dx: i32,
    pub mouse_dy: i32,
    // Normalized knob position for the visualizer (-1.0 to 1.0), synced from VisualState
    pub knob_x: f32,
    pub knob_y: f32,
}

impl Default for UiState {
    fn default() -> Self {
        UiState {
            sensitivity_x: 1.0,
            sensitivity_y: 1.0,
            deadzone: 0.05,
            trigger_button: "Not bound".to_string(),
            grab_audio_enabled: true,
            release_audio_enabled: true,
            system_tray_enabled: false,
            device_path: None,
            device_list: Vec::new(),
            trigger_listening: false,
            device_dropdown_open: false,
            center_strength_x: 0.04,
            center_strength_y: 0.02,
            center_while_grabbed: true,
            grabbed: false,
            mouse_dx: 0,
            mouse_dy: 0,
            knob_x: 0.0,
            knob_y: 0.0,
        }
    }
}

use rune::{
    AppSpec, Color, Cx, Drag, El, KeyEvent, Key, TextStyle, Tx, Shadow,
    col, el, row, text,
};
use crate::state::AppState;
use crate::config::Config;
use std::sync::OnceLock;
use crate::device_manager;

// Text styles
// const TITLE_STYLE: TextStyle = TextStyle::new(14.0, 600.0);
const LABEL_STYLE: TextStyle = TextStyle::new(12.0, 500.0);
const VALUE_STYLE: TextStyle = TextStyle::new(12.0, 450.0);
const SMALL_STYLE: TextStyle = TextStyle::new(11.0, 400.0);

impl From<AppState> for UiState {
    fn from(state: AppState) -> Self {
        UiState {
            sensitivity_x: state.sensitivity_x,
            sensitivity_y: state.sensitivity_y,
            deadzone: state.deadzone,
            trigger_button: state.trigger_button,
            grab_audio_enabled: state.grab_audio_enabled,
            release_audio_enabled: state.release_audio_enabled,
            system_tray_enabled: state.minimize_to_tray,
            device_path: state.device_path,
            device_list: Vec::new(),
            trigger_listening: state.trigger_listening,
            device_dropdown_open: false,
            center_strength_x: state.center_strength_x,
            center_strength_y: state.center_strength_y,
            center_while_grabbed: state.center_while_grabbed,
            grabbed: state.grabbed,
            mouse_dx: state.mouse_dx,
            mouse_dy: state.mouse_dy,
            knob_x: 0.0,
            knob_y: 0.0,
        }
    }
}

impl From<UiState> for AppState {
    fn from(ui: UiState) -> Self {
        AppState {
            device_path: ui.device_path,
            sensitivity_x: ui.sensitivity_x,
            sensitivity_y: ui.sensitivity_y,
            deadzone: ui.deadzone,
            trigger_button: ui.trigger_button,
            minimize_to_tray: ui.system_tray_enabled,
            grab_audio_enabled: ui.grab_audio_enabled,
            release_audio_enabled: ui.release_audio_enabled,
            center_strength_x: ui.center_strength_x,
            center_strength_y: ui.center_strength_y,
            center_while_grabbed: ui.center_while_grabbed,
            grabbed: ui.grabbed,
            mouse_dx: 0,
            mouse_dy: 0,
            trigger_listening: ui.trigger_listening,
        }
    }
}

// Global holder for initial state (since init must be fn())
static INITIAL_STATE: OnceLock<UiState> = OnceLock::new();

// Channel to send UI state updates back to main
static UI_TX: OnceLock<tokio::sync::mpsc::UnboundedSender<AppState>> = OnceLock::new();

pub fn run_ui(initial_state: AppState, ui_tx: tokio::sync::mpsc::UnboundedSender<AppState>) -> anyhow::Result<()> {
    log::info!("Starting mouse2joy UI...");

    // Store the sender for later use
    let _ = UI_TX.set(ui_tx);

    // Convert to UI state and store in global for the init fn
    let mut ui_state: UiState = initial_state.into();
    // Populate device list
    ui_state.device_list = device_manager::enumerate_mice();
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

fn poll_shared_state(state: &mut UiState, _now: f64, _tx: &mut Tx) -> bool {
    // Read the latest knob position and grab state from the lock-free shared
    // VisualState. This is updated by the joystick emulator task on every
    // frame, so the UI visualizer stays in sync without needing to await the
    // tokio Mutex.
    if let Some(visual) = VISUAL.get() {
        let (kx, ky) = visual.knob();
        state.knob_x = kx;
        state.knob_y = ky;
        state.grabbed = visual.is_grabbed();
    }
    true
}

fn on_key(state: &mut UiState, event: &KeyEvent, _tx: &mut Tx) -> bool {
    if state.trigger_listening {
        // Only Escape cancels trigger listening; mouse buttons are captured
        // by the device reader in device_manager.rs, not via Rune's on_key.
        match &event.key {
            Key::Escape => {
                state.trigger_button = "Not bound".to_string();
                state.trigger_listening = false;
                if let Some(tx) = UI_TX.get() {
                    let _ = tx.send(state.clone().into());
                }
                true
            }
            _ => false, // Ignore all other keys; mouse buttons captured via evdev
        }
    } else {
        false
    }
}

fn view(state: &UiState, _cx: &mut Cx<UiState>) -> El {
    // Pure black background
    let bg = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    let panel_bg = Color { r: 0.07, g: 0.07, b: 0.07, a: 1.0 };
    let panel_border = Color { r: 0.15, g: 0.15, b: 0.15, a: 1.0 };
    let accent = Color { r: 0.35, g: 0.55, b: 1.0, a: 1.0 };
    let text_primary = Color { r: 0.95, g: 0.95, b: 0.95, a: 1.0 };
    let text_secondary = Color { r: 0.6, g: 0.6, b: 0.65, a: 1.0 };
    let slider_track = Color { r: 0.12, g: 0.12, b: 0.15, a: 1.0 };
    let slider_knob = Color::WHITE;
    let toggle_on = accent;
    let toggle_off = Color { r: 0.25, g: 0.25, b: 0.3, a: 1.0 };
    let grabbed_color = Color { r: 0.2, g: 0.8, b: 0.3, a: 1.0 }; // Green for grabbed
    let ungrabbed_color = Color { r: 0.8, g: 0.3, b: 0.3, a: 1.0 }; // Red for ungrabbed

    // Left side: Joystick indicator (top) + Options (bottom)
    let left_panel = col()
        .grow(1.0)
        .h_full()
        .gap(20.0)
        .padding(20.0)
        .child(
            // Joystick indicator with grabbed status
            col()
                .gap(10.0)
                .child(
                    // Grabbed/Ungrabbed indicator
                    row()
                        .h(28.0)
                        .center()
                        .gap(8.0)
                        .child(
                            el()
                                .size(12.0, 12.0)
                                .radius(6.0)
                                .fill(if state.grabbed { grabbed_color } else { ungrabbed_color })
                                .box_shadow(Shadow::core_animation(0.0, 0.0, 8.0, if state.grabbed { grabbed_color } else { ungrabbed_color }))
                        )
                        .child(text(if state.grabbed { "GRABBED" } else { "UNGRABBED" })
                            .font(LABEL_STYLE)
                            .color(if state.grabbed { grabbed_color } else { ungrabbed_color }))
                )
                .child(
                    // Joystick indicator - larger circle, smaller dot
                    create_joystick_canvas(panel_bg, panel_border, accent, text_secondary, state.grabbed, state.knob_x, state.knob_y)
                )
        )
        .child(
            // Options - no box labels, just toggles
            col()
                .gap(14.0)
                .child(create_toggle(
                    "Grab Audio Cue", state.grab_audio_enabled,
                    text_primary, text_secondary, toggle_on, toggle_off,
                ))
                .child(create_toggle(
                    "Release Audio Cue", state.release_audio_enabled,
                    text_primary, text_secondary, toggle_on, toggle_off,
                ))
                .child(create_toggle(
                    "System Tray", state.system_tray_enabled,
                    text_primary, text_secondary, toggle_on, toggle_off,
                ))
                .child(create_toggle(
                    "Center While Grabbed", state.center_while_grabbed,
                    text_primary, text_secondary, toggle_on, toggle_off,
                ))
        );

    // Right side: Sensitivity settings (top) + Config (bottom)
    let right_panel = col()
        .grow(1.0)
        .h_full()
        .gap(20.0)
        .padding(20.0)
        .child(
            // Sensitivity settings - no section label
            col()
                .gap(16.0)
                .child(create_slider(
                    "X Sensitivity", state.sensitivity_x,
                    text_primary, text_secondary, accent, slider_track, slider_knob,
                ))
                .child(create_slider(
                    "Y Sensitivity", state.sensitivity_y,
                    text_primary, text_secondary, accent, slider_track, slider_knob,
                ))
                .child(create_slider(
                    "Deadzone", state.deadzone,
                    text_primary, text_secondary, accent, slider_track, slider_knob,
                ))
                .child(create_slider(
                    "Center Strength X", state.center_strength_x,
                    text_primary, text_secondary, accent, slider_track, slider_knob,
                ))
                .child(create_slider(
                    "Center Strength Y", state.center_strength_y,
                    text_primary, text_secondary, accent, slider_track, slider_knob,
                ))
        )
        .child(
            // Config - device selector, trigger button, import/export
            col()
                .gap(14.0)
                .child(create_device_selector(state, panel_bg, panel_border, text_primary, text_secondary, accent))
                .child(create_trigger_button(state, panel_bg, panel_border, text_primary, accent, text_secondary))
                .child(row()
                    .gap(12.0)
                    .child(create_button("Import", panel_bg, panel_border, text_primary, accent))
                    .child(create_button("Export", panel_bg, panel_border, text_primary, accent))
                )
        );

    // Main layout: two columns, no separator box/line
    col()
        .w_full()
        .h_full()
        .fill(bg)
        .child(
            row()
                .w_full()
                .h_full()
                .child(left_panel)
                .child(right_panel)
        )
}

fn create_joystick_canvas(
    _panel_bg: Color,
    panel_border: Color,
    _accent: Color,
    _text_secondary: Color,
    grabbed: bool,
    knob_x: f32,
    knob_y: f32,
) -> El {
    // Pure black background for joystick
    let bg = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    // Center position for joystick
    let center_x = 220.0;
    let center_y = 220.0;
    let stick_radius = 160.0;  // Larger circle
    let knob_radius = 16.0;    // Smaller dot
    let grabbed_color = Color { r: 0.2, g: 0.8, b: 0.3, a: 1.0 };
    let _ungrabbed_color = Color { r: 0.8, g: 0.3, b: 0.3, a: 1.0 };

    // Map normalized -1.0..=1.0 knob position to pixel offset within the stick.
    let travel = stick_radius - knob_radius;
    let knob_offset_x = knob_x * travel;
    let knob_offset_y = knob_y * travel;

    el()
        .key("joystick_canvas")
        .size(440.0, 440.0)
        .radius(12.0)
        .clip_rounded(12.0)
        .fill(bg)
        .border(2.0, if grabbed { grabbed_color } else { panel_border })
        .child(
            // Outer ring - larger
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
            // Knob - smaller dot in center, color changes based on grabbed state
            el()
                .absolute(center_x - knob_radius + knob_offset_x, center_y - knob_radius + knob_offset_y)
                .size(knob_radius * 2.0, knob_radius * 2.0)
                .radius(knob_radius)
                .fill(if grabbed { grabbed_color } else { Color { r: 0.35, g: 0.55, b: 1.0, a: 1.0 } })
        )
}

fn create_device_selector(
    state: &UiState,
    panel_bg: Color,
    panel_border: Color,
    text_primary: Color,
    text_secondary: Color,
    _accent: Color,
) -> El {
    let display_text = state.device_path
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("Select mouse device");

    let mut selector = col()
        .key("device_selector_container")
        .w_full()
        .gap(4.0);

    let header = row()
        .key("device_selector_header")
        .w_full()
        .h(44.0)
        .radius(8.0)
        .fill(panel_bg)
        .border(1.0, panel_border)
        .center_y()
        .px(16.0)
        .on_press(|s: &mut UiState| {
            s.device_dropdown_open = !s.device_dropdown_open;
        })
        .child(text(display_text).font(LABEL_STYLE).color(text_primary))
        .child(el().grow(1.0))
        .child(text(if state.device_dropdown_open { "▲" } else { "▼" }).font(SMALL_STYLE).color(text_secondary));

    selector = selector.child(header);

    if state.device_dropdown_open {
        let mut list = col()
            .key("device_list")
            .w_full()
            .fill(panel_bg)
            .border(1.0, panel_border)
            .radius(8.0)
            .padding(8.0)
            .gap(4.0);

        if state.device_list.is_empty() {
            list = list.child(text("No devices found").font(SMALL_STYLE).color(text_secondary));
        } else {
            for device in &state.device_list {
                let d = device.clone();
                list = list.child(
                    el()
                        .w_full()
                        .h(32.0)
                        .center_y()
                        .px(8.0)
                        .on_press(move |s: &mut UiState| {
                            s.device_path = Some(d.clone());
                            s.device_dropdown_open = false;
                            // Sync
                            if let Some(tx) = UI_TX.get() {
                                let _ = tx.send(s.clone().into());
                            }
                        })
                        .child(text(device).font(SMALL_STYLE).color(text_primary))
                );
            }
        }
        selector = selector.child(list);
    }

    selector
}

fn create_slider(
    label: &str,
    value: f32,
    text_primary: Color,
    text_secondary: Color,
    accent: Color,
    slider_track: Color,
    slider_knob: Color,
) -> El {
    let slider_width = 280.0;
    let slider_height = 24.0;
    let knob_diameter = 20.0;
    let bar_height = 6.0;
    let travel = slider_width - knob_diameter;
    let knob_x = knob_diameter / 2.0 + value * travel;
    let fill_width = knob_diameter / 2.0 + value * travel;
    let label_key = label.replace(" ", "_").to_lowercase();

    row()
        .w_full()
        .h(56.0)
        .gap(16.0)
        .center_y()
        .child(text(label).font(LABEL_STYLE).color(text_primary).w(130.0))
        .child(
            el()
                .key(&format!("slider_{}", label_key))
                .role("slider")
                .label(label)
                .size(slider_width, slider_height)
                .on_drag(move |s: &mut UiState, d: &mut Drag, _tx: &mut Tx| {
                    let new_value = ((d.x - knob_diameter / 2.0) / travel).clamp(0.0, 1.0);
                    match label_key.as_str() {
                        "x_sensitivity" => s.sensitivity_x = new_value,
                        "y_sensitivity" => s.sensitivity_y = new_value,
                        "deadzone" => s.deadzone = new_value,
                        "center_strength_x" => s.center_strength_x = new_value,
                        "center_strength_y" => s.center_strength_y = new_value,
                        _ => {}
                    }
                    // Sync to shared AppState on each drag update
                    if let Some(tx) = UI_TX.get() {
                        let _ = tx.send(s.clone().into());
                    }
                })
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
                        .box_shadow(Shadow::core_animation(0.0, 2.0, 6.0, Color { r: 0.0, g: 0.0, b: 0.0, a: 0.4 }))
                )
        )
        .child(text(format!("{:.0}%", value * 100.0)).font(VALUE_STYLE).color(text_secondary).w(60.0))
}

fn create_trigger_button(
    state: &UiState,
    panel_bg: Color,
    panel_border: Color,
    _text_primary: Color,
    accent: Color,
    text_secondary: Color,
) -> El {
    let btn_text = if state.trigger_listening {
        "Press key or pick below..."
    } else {
        &state.trigger_button
    };
    let is_listening = state.trigger_listening;
    let (btn_bg, btn_border, text_color) = if is_listening {
        (accent, accent, Color::WHITE)
    } else {
        (panel_bg, panel_border, accent)
    };

    let btn = el()
        .key("trigger_button")
        .w_full()
        .h(48.0)
        .radius(8.0)
        .fill(btn_bg)
        .border(2.0, btn_border)
        .center_y()
        .px(16.0)
        .on_press(|s: &mut UiState| {
            s.trigger_listening = !s.trigger_listening;
            // Sync to shared AppState
            if let Some(tx) = UI_TX.get() {
                let _ = tx.send(s.clone().into());
            }
        })
        .child(text("Trigger Button").font(SMALL_STYLE).color(text_secondary).absolute(16.0, 6.0))
        .child(text(btn_text).font(LABEL_STYLE).color(text_color).absolute(16.0, 26.0));

    let mut container = col()
        .key("trigger_container")
        .w_full()
        .gap(4.0)
        .child(btn);

    if state.trigger_listening {
        let menu = col()
            .key("trigger_menu")
            .w_full()
            .fill(panel_bg)
            .border(1.0, panel_border)
            .radius(8.0)
            .padding(8.0)
            .gap(2.0)
            .child(text("Press a mouse button, or pick below. Esc = Not bound.").font(SMALL_STYLE).color(text_secondary))
            .child(trigger_option_row("Mouse Side (4)", "BTN_SIDE".to_string(), accent, text_secondary))
            .child(trigger_option_row("Mouse Forward (5)", "BTN_FORWARD".to_string(), accent, text_secondary))
            .child(trigger_option_row("Middle Click", "BTN_MIDDLE".to_string(), accent, text_secondary))
            .child(trigger_option_row("Mouse Extra (6)", "BTN_EXTRA".to_string(), accent, text_secondary))
            .child(trigger_option_row("Task Button", "BTN_TASK".to_string(), accent, text_secondary))
            .child(trigger_option_row("Not bound (Esc)", "Not bound".to_string(), accent, text_secondary));
        container = container.child(menu);
    }

    container
}

fn trigger_option_row(label: &str, value: String, accent: Color, _text_secondary: Color) -> El {
    el()
        .w_full()
        .h(32.0)
        .center_y()
        .px(8.0)
        .on_press(move |s: &mut UiState| {
            s.trigger_button = value.clone();
            s.trigger_listening = false;
            // Sync to shared AppState
            if let Some(tx) = UI_TX.get() {
                let _ = tx.send(s.clone().into());
            }
        })
        .child(text(label).font(SMALL_STYLE).color(accent))
}

fn create_toggle(
    label: &str,
    enabled: bool,
    text_primary: Color,
    text_secondary: Color,
    toggle_on: Color,
    toggle_off: Color,
) -> El {
    let (track_color, thumb_x) = if enabled {
        (toggle_on, 32.0)
    } else {
        (toggle_off, 4.0)
    };

    let label_key = label.replace(" ", "_").to_lowercase();

    row()
        .w_full()
        .h(44.0)
        .gap(16.0)
        .center_y()
        .child(text(label).font(LABEL_STYLE).color(text_primary).grow(1.0))
        .child(
            el()
                .size(56.0, 32.0)
                .radius(16.0)
                .fill(track_color)
                .on_press(move |s: &mut UiState| {
                    match label_key.as_str() {
                        "grab_audio_cue" => s.grab_audio_enabled = !s.grab_audio_enabled,
                        "release_audio_cue" => s.release_audio_enabled = !s.release_audio_enabled,
                        "system_tray" => s.system_tray_enabled = !s.system_tray_enabled,
                        "center_while_grabbed" => s.center_while_grabbed = !s.center_while_grabbed,
                        _ => {}
                    }
                    // Sync to shared AppState
                    if let Some(tx) = UI_TX.get() {
                        let _ = tx.send(s.clone().into());
                    }
                })
                .child(
                    el()
                        .absolute(thumb_x, 2.0)
                        .size(28.0, 28.0)
                        .radius(14.0)
                        .fill(Color::WHITE)
                        .box_shadow(Shadow::core_animation(0.0, 2.0, 6.0, Color { r: 0.0, g: 0.0, b: 0.0, a: 0.4 }))
                )
        )
        .child(text(if enabled { "ON" } else { "OFF" }).font(VALUE_STYLE).color(if enabled { toggle_on } else { text_secondary }))
}

fn create_button(
    label: &str,
    panel_bg: Color,
    panel_border: Color,
    text_primary: Color,
    _accent: Color,
) -> El {
    let label_key = label.replace(" ", "_").to_lowercase();

    row()
        .h(32.0)
        .center()
        .radius(6.0)
        .fill(panel_bg)
        .border(1.0, panel_border)
        .px(16.0)
        .on_press(move |s: &mut UiState| {
            match label_key.as_str() {
                "import" => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("TOML", &["toml"])
                        .pick_file() {
                        if let Ok(cfg) = load_config_from_path(&path) {
                            s.sensitivity_x = cfg.sensitivity_x;
                            s.sensitivity_y = cfg.sensitivity_y;
                            s.deadzone = cfg.deadzone;
                            s.trigger_button = cfg.trigger_button;
                            s.grab_audio_enabled = cfg.grab_audio_enabled;
                            s.release_audio_enabled = cfg.release_audio_enabled;
                            s.system_tray_enabled = cfg.minimize_to_tray;
                            s.device_path = cfg.device_path;
                            s.center_strength_x = cfg.center_strength_x;
                            s.center_strength_y = cfg.center_strength_y;
                            s.center_while_grabbed = cfg.center_while_grabbed;
                            // Sync to shared AppState
                            if let Some(tx) = UI_TX.get() {
                                let _ = tx.send(s.clone().into());
                            }
                        }
                    }
                }
                "export" => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("TOML", &["toml"])
                        .save_file() {
                        let cfg = Config {
                            device_path: s.device_path.clone(),
                            sensitivity_x: s.sensitivity_x,
                            sensitivity_y: s.sensitivity_y,
                            deadzone: s.deadzone,
                            trigger_button: s.trigger_button.clone(),
                            minimize_to_tray: s.system_tray_enabled,
                            grab_audio_enabled: s.grab_audio_enabled,
                            release_audio_enabled: s.release_audio_enabled,
                            center_strength_x: s.center_strength_x,
                            center_strength_y: s.center_strength_y,
                            center_while_grabbed: s.center_while_grabbed,
                        };
                        let _ = save_config_to_path(&cfg, &path);
                    }
                }
                _ => {}
            }
        })
        .child(text(label).font(SMALL_STYLE).color(text_primary))
}

// enumerate_mice is defined in device_manager.rs

fn load_config_from_path(path: &std::path::Path) -> anyhow::Result<Config> {
    let content = std::fs::read_to_string(path)?;
    let cfg: Config = toml::from_str(&content)?;
    Ok(cfg)
}

fn save_config_to_path(cfg: &Config, path: &std::path::Path) -> anyhow::Result<()> {
    let toml_str = toml::to_string_pretty(cfg)?;
    std::fs::write(path, toml_str)?;
    Ok(())
}