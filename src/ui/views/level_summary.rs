use egui::{Button, CentralPanel, Context, Grid, ScrollArea};
use crate::model::{AppState, Language};
use crate::QuizApp;

pub fn ui_level_summary(app: &mut QuizApp, ctx: &Context) {
    CentralPanel::default().show(ctx, |ui| {
        let max_width = 600.0;
        let panel_width = (ui.available_width() * 0.97).min(max_width);
        let button_width = panel_width / 3.0;
        let button_height = 36.0;
        let total_height = 500.0;
        let extra_space = (ui.available_height() - total_height).max(0.0) / 2.0;

        ui.add_space(extra_space);

        ui.vertical_centered(|ui| {
            egui::Frame::default()
                .fill(ui.visuals().window_fill())
                .inner_margin(egui::Margin::symmetric(16, 50))
                .show(ui, |ui| {
                    ui.set_width(panel_width / 1.5);

                    ui.heading("Resumen de Nivel");
                    ui.add_space(10.0);
                    ui.label("Estado de las preguntas de este nivel:");
                    ui.add_space(5.0);

                    // Rejilla de detalle de preguntas
                    let max_height = 500.0;
                    ScrollArea::vertical()
                        .max_height(max_height)
                        .max_width(panel_width)
                        .show(ui, |ui| {
                            let wi = app.progress().current_week.unwrap_or(0);
                            let li = app.progress().current_level.unwrap_or(0);
                            let lang = app.selected_language.unwrap_or(Language::C);
                            let completed = &app.progress().completed_ids;

                            if let Some(level) = app.quiz.weeks.get(wi)
                                .and_then(|w| w.levels.get(li)) {
                                Grid::new("level_results_grid")
                                    .striped(true)
                                    .spacing([8.0, 0.0])
                                    .show(ui, |ui| {
                                        ui.label("Pregunta");
                                        ui.label("Intentos");
                                        ui.label("Fallos");
                                        ui.label("Saltos");
                                        ui.label("Solución vista");
                                        ui.label("Estado");
                                        ui.end_row();

                                        // Solo preguntas del lenguaje filtrar + enumerar
                                        for (i, q) in level
                                            .questions
                                            .iter()
                                            .filter(|q| q.language == lang)
                                            .enumerate()
                                        {
                                            // ¿Completada según completed_ids?
                                            let done = q.id.as_ref()
                                                .map(|id| completed.contains(id))
                                                .unwrap_or(false);
                                            let status = if done {
                                                "✅ Correcta"
                                            } else if q.saw_solution {
                                                "❌ Fallida"
                                            } else {
                                                "❌ Sin responder"
                                            };
                                            let solv = if q.saw_solution { "Sí" } else { "No" };

                                            ui.label(format!("{}", i + 1));
                                            ui.label(format!("{}", q.attempts));
                                            ui.label(format!("{}", q.fails));
                                            ui.label(format!("{}", q.skips));
                                            ui.label(solv);
                                            ui.label(status);
                                            ui.end_row();
                                        }
                                    });
                            } else {
                                ui.label("No hay datos para este nivel.");
                            }
                        });

                    ui.add_space(10.0);

                    // Botones de control
                    ui.vertical_centered(|ui| {
                        let wi = app.progress().current_week.unwrap_or(0);
                        let li = app.progress().current_level.unwrap_or(0);
                        let levels = &app.quiz.weeks[wi].levels;
                        let total_levels = levels.len();
                        let is_level_done = app.is_level_completed(wi, li);
                        let has_next_level = li + 1 < total_levels;
                        let week_done = app.is_week_completed(wi);
                        let has_next_week = app.has_next_week();

                        if is_level_done && has_next_level {
                            if ui
                                .add_sized([button_width, button_height], Button::new("Siguiente Nivel"))
                                .clicked()
                            {
                                app.select_level(wi, li + 1);
                                app.state = AppState::Quiz;

                                //app.avanzar_a_siguiente_nivel();
                            }
                        } else if week_done && has_next_week {
                            ui.add_space(10.0);
                            ui.label("¡Bien hecho! Has completado todos los niveles de esta semana.");
                            ui.add_space(10.0);
                            if ui
                                .add_sized([button_width, button_height], Button::new("Siguiente semana"))
                                .clicked()
                            {
                                app.avanzar_a_siguiente_semana();
                            }
                        } else if week_done && !has_next_week {
                            ui.add_space(10.0);
                            ui.label("¡Bien hecho! Has acabado el quiz.");
                            ui.add_space(10.0);
                            if ui
                                .add_sized([button_width, button_height], Button::new("Volver"))
                                .clicked()
                            {
                                app.state = AppState::WeekMenu;
                            }
                        } else {
                            if ui
                                .add_sized([button_width, button_height], Button::new("Atrás"))
                                .clicked()
                            {
                                app.state = AppState::Quiz;
                            }
                        }
                    });
                });
        });
    });
}
