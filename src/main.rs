#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use byteorder::{ByteOrder, LittleEndian};
use daytime_population::DaytimePopulationApp;
use daytime_population::POPULATION_COUNT;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    use eframe::egui;
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let population_count_bytes = include_bytes!("../assets/population_count.bin");
    let mut data = vec![0; 1440 * 720];
    LittleEndian::read_u64_into(population_count_bytes, data.as_mut_slice());
    POPULATION_COUNT.set(data).unwrap();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Daylight Population",
        native_options,
        Box::new(|cc| {
            let style = egui::Style {
                visuals: egui::Visuals::dark(),
                ..egui::Style::default()
            };
            cc.egui_ctx.set_style(style);
            Ok(Box::new(DaytimePopulationApp::new(cc)))
        }),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::egui;
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let population_count_bytes = include_bytes!("../assets/population_count.bin");
    let mut data = vec![0; 1440 * 720];
    LittleEndian::read_u64_into(population_count_bytes, data.as_mut_slice());
    POPULATION_COUNT.set(data).unwrap();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| {
                    let style = egui::Style {
                        visuals: egui::Visuals::dark(),
                        ..egui::Style::default()
                    };
                    cc.egui_ctx.set_style(style);
                    Ok(Box::new(DaytimePopulationApp::new(cc)))
                }),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
