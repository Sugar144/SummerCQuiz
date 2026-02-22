use crate::QuizApp;
use crate::model::Language;
use egui::{Align, Button, CentralPanel, Context, RichText};

pub fn ui_welcome(app: &mut QuizApp, ctx: &Context) {
    CentralPanel::default().show(ctx, |ui| {
        let max_width = 540.0;
        let content_width = ui.available_width().min(max_width);

        // Centrar verticalmente
        let estimated_h = 230.0;
        let vs = ((ui.available_height() - estimated_h) / 2.0).max(0.0);
        ui.add_space(vs / 2.0);

        ui.horizontal_centered(|ui| {
            egui::Frame::default()
                .fill(ui.visuals().window_fill())
                .inner_margin(egui::Margin::symmetric(16, 16))
                .show(ui, |ui| {
                    ui.with_layout(egui::Layout::top_down(Align::Center), |ui| {
                        ui.heading("¿Qué deseas hacer?");
                        ui.add_space(18.0);

                        // ¿Hay progreso guardado y preguntas pendientes?
                        let lang = app.selected_language.unwrap_or(Language::C);
                        let hay_guardado = app.has_saved_progress;
                        let hay_pendientes = app.quiz.modules
                            .iter()
                            .flat_map(|w| &w.levels)
                            .flat_map(|lvl| &lvl.questions)
                            .filter(|q| q.language == lang)
                            .any(|q| !q.is_done);

                        let btn_w = (content_width * 0.9).clamp(120.0, 400.0);
                        let btn_h = 40.0;

                        // Botones
                        let btn_cont = if hay_guardado && hay_pendientes {
                            Some(ui.add_sized([btn_w, btn_h], Button::new("▶ Continuar donde lo dejé")))
                        } else {
                            None
                        };
                        ui.add_space(5.0);
                        let btn_start = ui.add_sized([btn_w, btn_h], Button::new("🔄 Empezar de 0"));
                        ui.add_space(5.0);
                        let btn_menu  = ui.add_sized([btn_w, btn_h], Button::new("📅 Seleccionar Modulo"));
                        ui.add_space(5.0);
                        let btn_exit  = ui.add_sized([btn_w, btn_h], Button::new("🔙 Volver"));

                        if let Some(b) = btn_cont { if b.clicked() { app.continuar_quiz(); } }
                        if btn_start.clicked() {
                            if hay_guardado {
                                app.confirm_reset = true;
                            } else {
                                app.empezar_desde_cero();
                            }
                        }
                        if btn_menu.clicked() { app.abrir_menu_semanal(); }
                        if btn_exit.clicked() { app.salir_app(); }

                        // Confirmación de reinicio
                        if app.confirm_reset {
                            app.confirm_reset(ctx);
                        }

                        // Mensaje de nuevas preguntas (filtrar por completadas)
                        let nuevas = app.quiz.modules
                            .iter()
                            .filter(|w| app.is_module_completed(app.quiz.modules.iter().position(|wk| wk.number == w.number).unwrap_or(0)))
                            .flat_map(|w| &w.levels)
                            .flat_map(|lvl| &lvl.questions)
                            .filter(|q| q.language == lang)
                            .any(|q| !q.is_done);
                        if nuevas {
                            ui.add_space(10.0);
                            ui.label(
                                RichText::new("🟡 ¡Nuevas preguntas disponibles! Revisa las semanas completadas.")
                                    .color(egui::Color32::YELLOW)
                                    .heading()
                                    .strong(),
                            );
                        }
                    });
                });
        });

        ui.add_space(vs / 2.0);
    });
}
