use egui::{Button, CentralPanel, Context, Grid, ScrollArea};
use crate::model::{AppState, Language};
use crate::QuizApp;

pub fn ui_summary_view(app: &mut QuizApp, ctx: &Context) {
    CentralPanel::default().show(ctx, |ui| {
        let max_width = 600.0;
        let panel_width = (ui.available_width() * 0.97).min(max_width);
        let button_width = panel_width / 3.0;
        let button_height = 36.0;
        let total_height = 700.0;
        let extra_space = (ui.available_height() - total_height).max(0.0) / 2.0;

        ui.add_space(extra_space);

        ui.vertical_centered(|ui| {
            egui::Frame::default()
                .fill(ui.visuals().window_fill())
                .inner_margin(egui::Margin::symmetric(16, 50))
                .show(ui, |ui| {
                    ui.set_width(panel_width / 1.5);

                    ui.heading("Resumen de semana");
                    ui.add_space(10.0);
                    ui.label("Estado de las preguntas de esta semana");
                    ui.add_space(5.0);

                    let max_height = 700.0;
                    ScrollArea::vertical()
                        .max_height(max_height)
                        .max_width(panel_width)
                        .show(ui, |ui| {
                            // Extraer índice de semana actual
                            let wi = app.progress().current_week.unwrap_or(0);
                            if let Some(week) = app.quiz.weeks.get(wi) {
                                let lang = app.selected_language.unwrap_or(Language::C);
                                let completed = &app.progress().completed_ids;

                                Grid::new("quiz_results_grid")
                                    .striped(true)
                                    .spacing([8.0, 0.0])
                                    .show(ui, |ui| {
                                        ui.label("Nivel");
                                        ui.label("Pregunta");
                                        ui.label("Intentos");
                                        ui.label("Fallos");
                                        ui.label("Saltos");
                                        ui.label("Solución vista");
                                        ui.label("Estado");
                                        ui.end_row();

                                        for (li, level) in week.levels.iter().enumerate() {
                                            // solo preguntas del idioma actual
                                            for (qi, q) in level.questions.iter().enumerate().filter(|(_, q)| q.language == lang) {
                                                // estado según completed_ids
                                                let done = q.id.as_ref().map(|id| completed.contains(id)).unwrap_or(false);
                                                let status = if done {
                                                    "✅ Correcta"
                                                } else if q.saw_solution {
                                                    "❌ Fallida"
                                                } else {
                                                    "❌ Sin responder"
                                                };
                                                let solv = if q.saw_solution { "Sí" } else { "No" };

                                                ui.label(format!("{}", level.number));
                                                ui.label(format!("{}", qi + 1));
                                                ui.label(format!("{}", q.attempts));
                                                ui.label(format!("{}", q.fails));
                                                ui.label(format!("{}", q.skips));
                                                ui.label(solv);
                                                ui.label(status);
                                                ui.end_row();
                                            }
                                        }
                                    });
                            } else {
                                ui.label("No hay datos de progreso para esta semana.");
                            }
                        });

                    ui.add_space(5.0);

                    // Botones de control
                    ui.vertical_centered(|ui| {
                        let current_week = app.progress().current_week.unwrap_or(0);

                        let has_next = app.has_next_week();
                        let is_complete = app.is_week_completed(current_week);

                        if is_complete && has_next {
                            if ui.add_sized([button_width, button_height], Button::new("Siguiente Semana")).clicked() {
                                app.avanzar_a_siguiente_semana();
                            }
                        } else if is_complete {
                            ui.add_space(10.0);
                            ui.label("¡Bien hecho! Has acabado el quiz.");
                            ui.add_space(10.0);
                            if ui.add_sized([button_width, button_height], Button::new("Volver")).clicked() {
                                app.guardar_y_salir();
                            }
                        } else {
                            if ui.add_sized([button_width, button_height], Button::new("Atrás")).clicked() {
                                app.state = AppState::Quiz;
                            }
                        }
                    });
                });
        });
    });
}
