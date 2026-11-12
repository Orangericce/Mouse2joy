use rune::{App, Cx};
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn run_ui(_state: Arc<Mutex<crate::state::AppState>>) -> anyhow::Result<()> {
    log::info!("Starting mouse2joy UI...");

    let spec = rune::tooling::AppSpec {
        title: "mouse2joy",
        size: (1024.0, 600.0),
        min_size: None,
        init: || crate::state::AppState::new(crate::config::Config::default()),
        view: |_state: &crate::state::AppState, _cx: &mut Cx<crate::state::AppState>| -> rune::el::El {
            rune::el::col()
                .center()
                .padding(40.0)
                .gap(20.0)
                .child(
                    rune::el::text("mouse2joy - Virtual Joystick from Mouse")
                        .color(rune::color::Color::WHITE)
                )
                .child(
                    rune::el::text("Loading... Please wait for initialization")
                        .color(rune::color::Color::from_oklab([0.5, 0.0, 0.0, 1.0]))
                )
                .fill(rune::color::Color::from_oklab([0.1, 0.0, 0.0, 1.0]))
        },
        scenes: &[
            ("Main", move |_s: &mut crate::state::AppState| {}),
        ],
        fonts: &[],
        families: ("Inter", "Inter"),
        setup: None,
        poll: None,
        on_key: None,
    };

    let app = App::new(spec).keep_running();
    app.run();

    log::info!("UI completed");
    Ok(())
}