mod model;
mod app;
mod data;
mod code_utils;
mod update;
mod ui;

use app::QuizApp;

#[cfg(not(target_arch = "wasm32"))]
use egui::Visuals;
#[cfg(not(target_arch = "wasm32"))]
use crate::model::AppState;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            // Tamaño interior inicial
            .with_inner_size([1280.0, 1024.0])
            // Tamaño interior mínimo
            .with_min_inner_size([800.0, 600.0])
            // Deshabilita que el usuario redimensione la ventana
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "SummerQuiz - Telegram: @sugarRayL",
        options,
        Box::new(|cc| {
            // Detectar tema preferido del sistema
            let prefers_dark = cc.egui_ctx.style().visuals.dark_mode;
            if prefers_dark {
                cc.egui_ctx.set_visuals(Visuals::dark());
            } else {
                cc.egui_ctx.set_visuals(Visuals::light());
            }

            // === NUEVO BLOQUE: Cargar progreso si existe ===
            let app = if let Some(storage) = cc.storage {
                if let Some(mut state) = eframe::get_value::<QuizApp>(storage, eframe::APP_KEY) {
                    // Siempre empezar en select language:
                    state.state = AppState::LanguageSelect;
                    state.has_saved_progress = true;
                    state
                } else {
                    let mut app = QuizApp::new();
                    app.state = AppState::LanguageSelect;
                    app.has_saved_progress = false;
                    app
                }
            } else {
                let mut app = QuizApp::new();
                app.state = AppState::LanguageSelect;
                app.has_saved_progress = false;
                app
            };


            Ok(Box::new(app))
        }),
    )
}


// ======== SOLO PARA WEB/WASM ========
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirige logs de Rust a la consola JS
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

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
                Box::new(|_cc| Ok(Box::new(QuizApp::new()))),
                // ^--- O QuizApp::new() si tu constructor NO usa cc.
            )
            .await;

        // (Opcional) Elimina el texto de "Cargando..." si lo tienes en el HTML
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p>La app ha fallado. Mira la consola de desarrollador para detalles.</p>",
                    );
                    panic!("Fallo al iniciar eframe: {e:?}");
                }
            }
        }
    });
}
