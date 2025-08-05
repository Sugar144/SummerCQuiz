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
                    if let (Some(wi), Some(li), Some(qi)) = (
                        app.progress().current_week,
                        app.progress().current_level,
                        app.progress().current_in_level,
                    ) {
                        let question = app.quiz.weeks[wi].levels[li].questions[qi].clone();

                        // Ronda
                        ui.heading(format!("🌀 Ronda {}", app.progress().round));
                        ui.add_space(10.0);

                        // Prompt con scroll fijo
                        let prompt_text = &question.prompt;
                        let prompt_max_h = 150.0;
                        let prompt_min_lines = 4.0;
                        let font_id = egui::TextStyle::Body.resolve(ui.style());
                        let line_h = ui.fonts(|f| f.row_height(&font_id));
                        let prompt_min_h = prompt_min_lines * line_h;
                        let galley = ui.fonts(|fonts| fonts.layout(
                            prompt_text.clone(), font_id.clone(), egui::Color32::WHITE, panel_width));
                        let needed_h = galley.size().y.max(prompt_min_h).min(prompt_max_h);
                        ui.allocate_ui_with_layout(
                            egui::vec2(panel_width, needed_h),
                            egui::Layout::top_down(Align::Min),
                            |ui| {
                                ScrollArea::vertical()
                                    .max_height(prompt_max_h)
                                    .show(ui, |ui| { ui.label(prompt_text); });
                            },
                        );

                        ui.add_space(5.0);

                        // Editor de código o solución
                        let language = app.selected_language.unwrap_or(Language::C);
                        let syntax = match language {
                            Language::C => c_syntax(),
                            Language::Pseudocode => pseudo_syntax(),
                        };
                        let max_input_h = 245.0;
                        let min_lines = 15;
                        let font_id = egui::TextStyle::Monospace.resolve(ui.style());
                        let line_h = ui.fonts(|f| f.row_height(&font_id));
                        let code_rows = min_lines;

                        if question.fails >= 2 {
                            if !app.progress().show_solution {
                                if ui.button("Solución").clicked() {
                                    app.progress_mut().show_solution = true;
                                }
                                code_editor_input(
                                    ui, "user_input", panel_width, code_rows, line_h,
                                    syntax.clone(), &mut app.progress_mut().input, max_input_h,
                                );
                            } else {
                                if ui.button("Siguiente pregunta").clicked() {
                                    app.avanzar_a_siguiente_pregunta();
                                }
                                code_editor_solution(
                                    ui, panel_width, code_rows, line_h,
                                    syntax, &question.answer, max_input_h,
                                );
                            }
                        } else {
                            code_editor_input(
                                ui, "user_input", panel_width, code_rows, line_h,
                                syntax, &mut app.progress_mut().input, max_input_h,
                            );
                        }

                        // Pista si falla
                        if question.fails >= 1 {
                            if let Some(hint) = &question.hint {
                                ui.label(format!("💡 Pista: {}", hint));
                            }
                        }

                        ui.add_space(5.0);

                        // Botón de test: marcar semana completa
                        if ui.button("⚡ Marcar semana como completada (TEST)").clicked() {
                            if let Some(wi) = app.progress().current_week {
                                let lang = app.selected_language.unwrap_or(Language::C);
                                let mut ids_to_mark = Vec::new();

                                // 1) Marcar todas las preguntas de esta semana como hechas
                                for level in &mut app.quiz.weeks[wi].levels {
                                    for q in &mut level.questions {
                                        if q.language == lang {
                                            q.is_done = true;
                                            q.saw_solution = false;
                                            q.attempts = 1;
                                            q.fails = 0;
                                            q.skips = 0;
                                            if let Some(id) = &q.id {
                                                ids_to_mark.push(id.clone());
                                            }
                                        }
                                    }
                                }

                                // 2) Insertar los IDs en completed_ids
                                {
                                    let prog = app.progress_mut();
                                    for id in ids_to_mark {
                                        prog.completed_ids.insert(id);
                                    }
                                }

                                // 3) Completar la semana y desbloquear la siguiente
                                app.complete_week(wi);
                                app.recalculate_unlocked_weeks();

                                // ─── ¡NUEVO! Asegúrate de propagar esos completed_ids a `q.is_done` en todas las preguntas ───
                                app.sync_is_done();

                                // 4) Ir al resumen
                                app.state = AppState::Summary;
                            }
                        }



                        // Botones enviar/saltar
                        let (enviar, saltar) = two_button_row(ui, panel_width, "Enviar", "Saltar pregunta");
                        if enviar {
                            let input = app.progress().input.clone();
                            app.procesar_respuesta(&input);
                        }
                        if saltar {
                            app.saltar_pregunta();
                        }

                        // Volver / ver progreso
                        let (volver, progreso) = two_button_row(ui, panel_width, "Volver", "Ver progreso");
                        if progreso { app.ver_progreso(); }
                        if volver { app.guardar_y_salir(); }

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
