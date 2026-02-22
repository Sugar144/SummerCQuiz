use crate::QuizApp;
use crate::ui::layout::centered_panel;
use egui::{Context, RichText, Spinner};

pub fn ui_pending_update(app: &mut QuizApp, ctx: &Context) {
    // Panel central
    centered_panel(ctx, 300.0, 400.0, |ui| {
        ui.add_space(60.0);
        ui.label(
            RichText::new(&app.message)
                .heading()
                .color(egui::Color32::YELLOW),
        );
        ui.add_space(20.0);
        ui.add(Spinner::new());
    });

    // Lanzas el thread SOLO la primera vez
    app.ensure_update_thread();
}
