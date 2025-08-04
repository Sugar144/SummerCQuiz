use egui::{Align, Button, CentralPanel, Context, RichText};
use crate::QuizApp;

pub fn ui_welcome(app: &mut QuizApp, ctx: &Context) {
    CentralPanel::default().show(ctx, |ui| {
        let max_width = 540.0;
        let content_width = ui.available_width().min(max_width);

        // Calcular espacio vertical para centrar
        let estimated_height = 230.0;
        let vertical_space = ((ui.available_height() - estimated_height) / 2.0).max(0.0);
        ui.add_space(vertical_space / 2.0);

        ui.horizontal_centered(|ui| {
            egui::Frame::default()
                .fill(ui.visuals().window_fill())
                .inner_margin(egui::Margin::symmetric(16, 16))
                .show(ui, |ui| {
                    ui.with_layout(egui::Layout::top_down(Align::Center), |ui| {
                        ui.heading("¿Qué deseas hacer?");
                        ui.add_space(18.0);

                        let hay_guardado   = app.has_saved_progress;
                        let hay_pendientes = app.questions.iter().any(|q| !q.is_done);
                        let button_w       = (content_width * 0.9).clamp(120.0, 400.0);
                        let button_h       = 36.0;

                        let btn_continuar = if hay_guardado && hay_pendientes {
                            Some(ui.add_sized([button_w, button_h], Button::new("▶ Continuar donde lo dejé")))
                        } else {
                            None
                        };
                        let btn_empezar = ui.add_sized([button_w, button_h], Button::new("🔄 Empezar de 0"));
                        let btn_menu    = ui.add_sized([button_w, button_h], Button::new("📅 Seleccionar Semana"));
                        let btn_salir   = ui.add_sized([button_w, button_h], Button::new("🔙 Volver"));

                        // Manejo de clicks
                        if let Some(b) = btn_continuar {
                            if b.clicked() {
                                app.continuar_quiz();
                            }
                        }
                        if btn_empezar.clicked() {
                            if hay_guardado {
                                app.confirm_reset = true;
                            } else {
                                app.empezar_desde_cero();
                            }
                        }
                        if btn_menu.clicked() {
                            app.abrir_menu_semanal();
                        }
                        if btn_salir.clicked() {
                            app.salir_app();
                        }

                        // Ventana de confirmación de reinicio
                        if app.confirm_reset {
                            app.confirm_reset(ctx);
                        }

                        // Mensaje de nuevas preguntas
                        if app.hay_preguntas_nuevas() {
                            ui.label(
                                RichText::new("🟡 ¡Nuevas preguntas disponibles! Revisa las semanas completadas.")
                                    .color(egui::Color32::YELLOW)
                                    .heading()
                                    .strong()
                            );
                        }
                    });
                });
        });

        ui.add_space(vertical_space);
    });
}