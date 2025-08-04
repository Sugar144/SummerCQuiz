use egui::{Align, Button, CentralPanel, Context, RichText, Vec2};
use crate::model::Language;
use crate::QuizApp;

pub fn ui_week_menu(app: &mut QuizApp, ctx: &Context) {
    CentralPanel::default().show(ctx, |ui| {
        let max_width = 400.0;
        let content_width = ui.available_width().min(max_width);
        let button_h = 36.0;

        // Calcular nº de semanas para estimar altura
        let weeks_count = app.questions
            .iter()
            .filter(|q| q.language == app.selected_language.unwrap_or(Language::C))
            .map(|q| q.week)
            .collect::<std::collections::HashSet<_>>()
            .len();

        let estimated_height = 80.0 + (button_h + 8.0) * (weeks_count as f32 + 1.0);
        let vertical_space = ((ui.available_height() - estimated_height) / 2.0).max(0.0);

        ui.add_space(vertical_space / 2.0);

        ui.vertical_centered_justified(|ui| {
            egui::Frame::default()
                .fill(ui.visuals().window_fill())
                .inner_margin(egui::Margin::symmetric(24, 16))
                .show(ui, |ui| {
                    ui.with_layout(egui::Layout::top_down(Align::Center), |ui| {
                        ui.set_width(content_width);

                        ui.heading("Selecciona una semana");
                        ui.add_space(20.0);

                        // Mensaje destacado si existe
                        if !app.message.is_empty() {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(&app.message)
                                    .color(egui::Color32::YELLOW)
                                    .heading()
                                    .strong()
                            );
                            ui.add_space(8.0);
                        }

                        let language = app.selected_language.unwrap_or(Language::C);

                        // Lista de semanas únicas y ordenadas
                        let mut weeks: Vec<usize> = app.questions
                            .iter()
                            .filter(|q| q.language == language)
                            .map(|q| q.week)
                            .collect();
                        weeks.sort_unstable();
                        weeks.dedup();

                        for &week in &weeks {
                            let unlocked  = app.is_week_unlocked(week);
                            let completed = app.is_week_completed(week);
                            let n_nuevas  = app.nuevas_preguntas_en_semana(week, language);

                            let label = if completed && n_nuevas == 0 {
                                format!("Semana {} ✅", week)
                            } else if unlocked {
                                if n_nuevas > 0 {
                                    format!("Semana {} 🔓 ({} nuevas)", week, n_nuevas)
                                } else {
                                    format!("Semana {} 🔓", week)
                                }
                            } else {
                                format!("Semana {} 🔒", week)
                            };

                            ui.horizontal(|ui| {
                                if !completed {
                                    let enabled = unlocked;
                                    if ui.add_enabled(enabled, Button::new(&label)
                                        .min_size(Vec2::from([content_width, button_h])))
                                        .clicked() && enabled
                                    {
                                        app.acceder_a_semana(week);
                                    }
                                } else {
                                    // Botón de acceso reducido
                                    let enabled = unlocked;
                                    if ui.add_enabled(enabled, Button::new(&label)
                                        .min_size(Vec2::from([content_width * 0.75, button_h])))
                                        .clicked() && enabled
                                    {
                                        app.acceder_a_semana(week);
                                    }

                                    // Botón de reiniciar
                                    if ui.add_sized([content_width * 0.21, button_h], Button::new("↩")
                                        .fill(egui::Color32::DARK_RED))
                                        .on_hover_text("Reinicia la semana")
                                        .clicked()
                                    {
                                        app.reiniciar_semana(week);
                                        app.acceder_a_semana(week);
                                        return;
                                    }
                                }
                            });
                            ui.add_space(8.0);
                        }

                        ui.add_space(16.0);
                        if ui.add_sized([content_width, button_h], Button::new("Volver al menú principal"))
                            .clicked()
                        {
                            app.volver_al_menu_principal();
                        }
                    });
                });
        });

        ui.add_space(vertical_space);
    });
}