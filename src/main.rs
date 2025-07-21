mod model;
mod app;
mod data;
mod code_utils;
mod update;
mod ui;

use app::QuizApp;
use egui::Visuals;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
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

            let quiz = QuizApp::new();
            Ok(Box::new(quiz))
        }),
    )
}



