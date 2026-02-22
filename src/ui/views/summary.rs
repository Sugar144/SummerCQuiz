use crate::QuizApp;
use crate::model::AppState;
use crate::view_models::QuestionRow;
use egui::{Button, CentralPanel, Context, Grid, ScrollArea};

pub fn ui_summary_view(app: &mut QuizApp, ctx: &Context) {
    // Si no hay lenguaje, volvemos al selector para evitar panics en progress()
    if app.selected_language.is_none() {
        app.state = AppState::LanguageSelect;
        app.message = "Primero elige un lenguaje para ver el resumen.".to_owned();
        return;
    }

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
                            let rows: Vec<QuestionRow> = app.summary_rows_for_module();

                            if rows.is_empty() {
                                ui.label("No hay datos de progreso para esta semana.");
                                return;
                            }

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

                                    for r in &rows {
                                        ui.label(r.level_number.to_string());
                                        ui.label(r.question_index_1based.to_string());
                                        ui.label(r.attempts.to_string());
                                        ui.label(r.fails.to_string());
                                        ui.label(r.skips.to_string());
                                        ui.label(if r.saw_solution { "Sí" } else { "No" });
                                        ui.label(if r.done {
                                            "✅ Correcta"
                                        } else {
                                            "❌ Pendiente"
                                        });
                                        ui.end_row();
                                    }
                                });
                        });

                    ui.add_space(5.0);

                    // Botones de control
                    ui.vertical_centered(|ui| {
                        let current_module = app.progress().current_module.unwrap_or(0);
                        let has_next = app.has_next_module();
                        let is_complete = app.is_module_completed(current_module);

                        if is_complete && has_next {
                            if ui
                                .add_sized(
                                    [button_width, button_height],
                                    Button::new("Siguiente Semana"),
                                )
                                .clicked()
                            {
                                app.avanzar_a_siguiente_semana();
                            }
                        } else if is_complete {
                            ui.add_space(10.0);
                            ui.label("¡Bien hecho! Has acabado el quiz.");
                            ui.add_space(10.0);
                            if ui
                                .add_sized([button_width, button_height], Button::new("Volver"))
                                .clicked()
                            {
                                app.volver_niveles();
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
