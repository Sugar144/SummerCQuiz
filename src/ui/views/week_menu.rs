use egui::{Align, Button, CentralPanel, Context, RichText, Vec2};
use crate::model::Language;
use crate::QuizApp;

pub fn ui_week_menu(app: &mut QuizApp, ctx: &Context) {
    CentralPanel::default().show(ctx, |ui| {
        let max_width = 400.0;
        let content_width = ui.available_width().min(max_width);
        let button_h = 36.0;

        // Altura estimada para centrar
        let weeks_count = app.quiz.weeks.len() as f32;
        let estimated_h = 80.0 + (button_h + 8.0) * (weeks_count + 1.0);
        let vertical_space = ((ui.available_height() - estimated_h) / 2.0).max(0.0);
        ui.add_space(vertical_space / 2.0);

        // Precomputar datos de cada semana para no mantener el borrow en la iteración
        let lang = app.selected_language.unwrap_or(Language::C);
        let week_infos: Vec<(usize, usize, bool, bool, usize)> = app
            .quiz
            .weeks
            .iter()
            .enumerate()
            .map(|(wi, wk)| {
                let week_num = wk.number;
                let unlocked = app.is_week_unlocked(wi);
                let completed = app.is_week_completed(wi);
                let new_count = app.nuevas_preguntas_en_semana(wi, lang);
                (wi, week_num, unlocked, completed, new_count)
            })
            .collect();

        ui.vertical_centered_justified(|ui| {
            egui::Frame::default()
                .fill(ui.visuals().window_fill())
                .inner_margin(egui::Margin::symmetric(24, 16))
                .show(ui, |ui| {
                    ui.with_layout(egui::Layout::top_down(Align::Center), |ui| {
                        ui.set_width(content_width);
                        ui.heading("Selecciona una semana");
                        ui.add_space(20.0);

                        if !app.message.is_empty() {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(&app.message)
                                    .color(egui::Color32::YELLOW)
                                    .heading()
                                    .strong(),
                            );
                            ui.add_space(8.0);
                        }

                        // Mostrar cada semana usando los datos precomputados
                        for (wi, week_num, unlocked, completed, new_count) in &week_infos {
                            let label = if *completed && *new_count == 0 {
                                format!("Semana {} ✅", week_num)
                            } else if *unlocked {
                                if *new_count > 0 {
                                    format!("Semana {} 🔓 ({} nuevas)", week_num, new_count)
                                } else {
                                    format!("Semana {} 🔓", week_num)
                                }
                            } else {
                                format!("Semana {} 🔒", week_num)
                            };

                            ui.horizontal(|ui| {
                                // botón principal (abrir niveles)
                                if ui
                                    .add_enabled(*unlocked, Button::new(&label)
                                        .min_size(Vec2::new(content_width * if *completed { 0.75 } else { 1.0 }, button_h)))
                                    .clicked()
                                    && *unlocked
                                {
                                    // 1) Fijar la semana seleccionada
                                    {
                                        let prog = app.progress_mut();
                                        prog.current_week = Some(*wi);
                                    } // <- aquí soltamos el borrow_mut

                                    // 2) Abrir menú de niveles
                                    app.open_level_menu();
                                }

                                // reiniciar semana (solo si está completada)
                                if *completed {
                                    if ui
                                        .add_sized([content_width * 0.21, button_h], Button::new("↩").fill(egui::Color32::DARK_RED))
                                        .on_hover_text("Reinicia la semana")
                                        .clicked()
                                    {
                                        // 1) Reinicia la semana entera
                                        app.reiniciar_semana(*wi);

                                        // 2) Vuelve a fijar la misma semana (ya no está "completed" pero seguimos en ella)
                                        {
                                            let prog = app.progress_mut();
                                            prog.current_week = Some(*wi);
                                        }

                                        // 3) Y volvemos al menú de niveles
                                        app.open_level_menu();
                                        return;
                                    }
                                }
                            });

                            ui.add_space(8.0);
                        }

                        ui.add_space(16.0);
                        if ui
                            .add_sized([content_width, button_h], Button::new("Volver al menú principal"))
                            .clicked()
                        {
                            app.volver_al_menu_principal();
                        }
                    });
                });
        });

        ui.add_space(vertical_space / 2.0);
    });
}
