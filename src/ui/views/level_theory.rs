use crate::app::QuizApp;
use crate::model::AppState;
use crate::ui::layout::two_button_row;
use eframe::egui;
use egui_commonmark::CommonMarkViewer;

pub fn ui_level_theory(app: &mut QuizApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let max_width = 650.0;
        let panel_width = (ui.available_width() * 0.97).min(max_width);

        let total_height = 150.0 + 245.0 + 48.0 + 48.0 + 24.0;
        let extra_space = (ui.available_height() - total_height).max(0.0) / 2.0;

        ui.add_space(extra_space / 4.0);

        // Localiza semana/nivel actuales
        let (wi, li) = match (app.progress().current_week, app.progress().current_level) {
            (Some(w), Some(l)) => (w, l),
            _ => {
                ui.label("No hay nivel seleccionado.");
                return;
            }
        };

        // Seguridad de índices
        if wi >= app.quiz.weeks.len() || li >= app.quiz.weeks[wi].levels.len() {
            ui.label("Índices fuera de rango.");
            return;
        }

        // Evita doble borrow y copias innecesarias gigantes
        let week_num = app.quiz.weeks[wi].number;
        let theory = app.quiz.weeks[wi].levels[li]
            .explanation
            .get(&app.selected_language.unwrap())
            .cloned()
            .unwrap_or_else(|| "No hay teoría para este lenguaje".into());

        egui::Frame::default()
            .fill(ui.visuals().window_fill())
            .inner_margin(egui::Margin::symmetric(120, 20))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.set_width(panel_width);
                    ui.heading(format!("Semana {}, Nivel {}", week_num, li + 1));
                    ui.add_space(10.0);

                    // --- clave: reservar altura para los botones ---
                    let footer_h = 60.0; // alto reservado p/ los dos botones
                    let available_h = total_height;
                    let text_h = (available_h - footer_h).max(0.0);

                    egui::ScrollArea::vertical()
                        .max_height(text_h)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            CommonMarkViewer::new().show(ui, &mut app.cm_cache, &theory);
                        });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    let back_label = match app.theory_return_state {
                        AppState::Quiz => "⬅ Volver al quiz",
                        _ => "⬅ Volver a niveles",
                    };

                    // Si vienes del quiz => solo un botón (volver)
                    if matches!(app.theory_return_state, AppState::Quiz) {
                        // Botón único a ancho completo
                        if ui
                            .add_sized([panel_width / 2.0, 36.0], egui::Button::new(back_label))
                            .clicked()
                        {
                            app.state = AppState::Quiz;
                            app.message.clear();
                        }
                    } else {
                        // Origen: menú de niveles => dos botones (volver | comenzar)
                        let (volver, comenzar) =
                            two_button_row(ui, panel_width, back_label, "Comenzar preguntas ▶");

                        if volver {
                            app.state = AppState::LevelMenu;
                            app.message.clear();
                        }
                        if comenzar {
                            {
                                let prog = app.progress_mut();
                                if prog.current_in_level.is_none() {
                                    prog.current_in_level = Some(0);
                                }
                                prog.finished = false;
                                prog.input.clear();
                            }
                            app.update_input_prefill();
                            app.state = AppState::Quiz;
                            app.message.clear();
                        }
                    }
                });
            });
    });
}
