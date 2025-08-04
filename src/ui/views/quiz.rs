use egui::{Align, CentralPanel, Context, ScrollArea};
use crate::code_utils::{c_syntax, pseudo_syntax};
use crate::model::{AppState, Language};
use crate::QuizApp;
use crate::ui::layout::{code_editor_input, code_editor_solution, two_button_row};

pub fn ui_quiz(app: &mut QuizApp, ctx: &Context) {
    CentralPanel::default().show(ctx, |ui| {
        let max_width = 650.0;
        let panel_width = (ui.available_width() * 0.97).min(max_width);
        let total_height = 150.0 + 245.0 + 48.0 + 48.0 + 24.0;
        let extra_space = (ui.available_height() - total_height).max(0.0) / 2.0;
        ui.add_space(extra_space / 4.0);

        egui::Frame::default()
            .fill(ui.visuals().window_fill())
            .inner_margin(egui::Margin::symmetric(120, 20))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    if let Some(idx) = app.progress().current_in_week {
                        ui.heading(format!("🌀 Ronda {}", app.progress().round));
                        ui.add_space(10.0);
                        // Prompt con scroll fijo
                        let prompt_max_height = 150.0;
                        let prompt_min_lines = 4.0;
                        let font_id = egui::TextStyle::Body.resolve(ui.style());
                        let line_height = ui.fonts(|f| f.row_height(&font_id));
                        let prompt_min_height = prompt_min_lines * line_height;
                        let prompt_text = app.questions[idx].prompt.clone();
                        let galley = ui.fonts(|fonts| {
                            fonts.layout(
                                prompt_text.clone(),
                                font_id.clone(),
                                egui::Color32::WHITE,
                                panel_width,
                            )
                        });
                        let needed_height = galley.size().y.max(prompt_min_height).min(prompt_max_height);
                        ui.allocate_ui_with_layout(
                            egui::vec2(panel_width, needed_height),
                            egui::Layout::top_down(Align::Min),
                            |ui| {
                                ScrollArea::vertical()
                                    .max_height(prompt_max_height)
                                    .show(ui, |ui| {
                                        ui.label(&prompt_text);
                                    });
                            }
                        );

                        ui.add_space(5.0);

                        let max_input_height = 245.0;
                        let min_lines = 15;
                        let font_id = egui::TextStyle::Monospace.resolve(ui.style());
                        let line_height = ui.fonts(|f| f.row_height(&font_id));
                        let code_rows = min_lines;

                        let syntax = match app.selected_language.unwrap_or(Language::C) {
                            Language::C => c_syntax(),
                            Language::Pseudocode => pseudo_syntax(),
                        };

                        if app.questions[idx].fails >= 2 {
                            if !app.progress().show_solution {
                                // Botón para mostrar solución
                                if ui.button("Solución").clicked() {
                                    app.progress_mut().show_solution = true;
                                }
                                // Editor de usuario
                                code_editor_input(
                                    ui,
                                    "user_input",
                                    panel_width,
                                    code_rows,
                                    line_height,
                                    syntax.clone(),
                                    &mut app.progress_mut().input,
                                    max_input_height,
                                );
                            } else {
                                // Botón “Siguiente pregunta”
                                if ui.button("Siguiente pregunta").clicked() {
                                    app.avanzar_a_siguiente_pregunta(idx);
                                }
                                // Editor de solo lectura con solución
                                code_editor_solution(
                                    ui,
                                    panel_width,
                                    code_rows,
                                    line_height,
                                    syntax,
                                    &app.questions[idx].answer,
                                    max_input_height,
                                );
                            }
                        } else {
                            // Editor estándar de usuario
                            code_editor_input(
                                ui,
                                "user_input",
                                panel_width,
                                code_rows,
                                line_height,
                                syntax,
                                &mut app.progress_mut().input,
                                max_input_height,
                            );
                        }

                        if app.questions[idx].fails >= 1 {
                            if let Some(hint) = &app.questions[idx].hint {
                                ui.label(format!("💡 Pista: {hint}"));
                            }
                        }

                        ui.add_space(5.0);

                        if ui.button("⚡ Marcar semana como completada (TEST)").clicked() {
                            let week = app.progress().current_week.unwrap_or(1);
                            let language = app.selected_language.unwrap_or(Language::C);
                            let mut ids_a_marcar = Vec::new();
                            for q in app.questions.iter_mut() {
                                if q.week == week && q.language == language {
                                    q.is_done = true;
                                    q.saw_solution = false;
                                    q.attempts = 1;
                                    q.fails = 0;
                                    q.skips = 0;
                                    if let Some(id) = &q.id {
                                        ids_a_marcar.push(id.clone());
                                    }
                                }
                            }
                            let next_idx = app.next_pending_in_week();
                            {
                                let progress = app.progress_mut();
                                for id in ids_a_marcar {
                                    progress.completed_ids.insert(id);
                                }
                                progress.current_in_week = next_idx;
                            }
                            if app.progress().current_in_week.is_none() {
                                app.state = AppState::Summary;
                            }
                        }

                        let (enviar, saltar) = two_button_row(ui, panel_width, "Enviar", "Saltar pregunta");
                        if enviar {
                            if let Some(idx) = app.progress().current_in_week {
                                let input = app.progress().input.clone();
                                app.procesar_respuesta(&input, idx);
                            }
                        }
                        if saltar {
                            app.saltar_pregunta();
                        }

                        // y lo mismo para “Volver” / “Ver progreso”:
                        let (volver, progreso) =
                            two_button_row(ui, panel_width, "Volver", "Ver progreso");
                        if progreso {
                            app.ver_progreso();
                        }
                        if volver {
                            app.guardar_y_salir();
                        }

                        ui.add_space(8.0);
                        if !app.message.is_empty() {
                            ui.label(&app.message);
                        }
                    }
                });
            });

        ui.add_space(extra_space);
    });
}