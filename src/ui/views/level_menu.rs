// src/ui/views/level_menu.rs
use egui::{Align, Button, CentralPanel, Context, RichText, Vec2};
use crate::model::Language;
use crate::QuizApp;

/// Menu para escoger nivel dentro de la semana seleccionada
pub fn ui_level_menu(app: &mut QuizApp, ctx: &Context) {
    CentralPanel::default().show(ctx, |ui| {
        let max_width = 400.0;
        let content_width = ui.available_width().min(max_width);
        let button_h = 36.0;

        // Obtener la semana actual (índice en quiz.weeks)
        let week_idx = match app.progress().current_week {
            Some(w) => w,
            None => return, // Si no hay semana seleccionada, no pintamos nada aquí
        };

        // Calcular altura estimada
        let levels_count = app.quiz.weeks[week_idx].levels.len() as f32;
        let estimated_h = 80.0 + (button_h + 8.0) * (levels_count + 1.0);
        let vertical_space = ((ui.available_height() - estimated_h) / 2.0).max(0.0);
        ui.add_space(vertical_space / 2.0);

        // Precomputar datos de cada nivel para evitar borrows múltiples
        let lang = app.selected_language.unwrap_or(Language::C);
        let level_infos: Vec<(usize, usize, bool, bool, usize)> = app.quiz.weeks[week_idx]
            .levels
            .iter()
            .enumerate()
            .map(|(li, lvl)| {
                let level_num = lvl.number;
                let unlocked = app.is_level_unlocked(week_idx, li);
                let completed = app.is_level_completed(week_idx, li);
                // Contar preguntas pendientes en este nivel
                let new_count = app.quiz.weeks[week_idx].levels[li]
                    .questions
                    .iter()
                    .filter(|q| q.language == lang)
                    .filter(|q| {
                        q.id
                            .as_ref()
                            .map(|id| !app.progress().completed_ids.contains(id))
                            .unwrap_or(false)
                    })
                    .count();
                (li, level_num, unlocked, completed, new_count)
            })
            .collect();

        ui.vertical_centered_justified(|ui| {
            egui::Frame::default()
                .fill(ui.visuals().window_fill())
                .inner_margin(egui::Margin::symmetric(24, 16))
                .show(ui, |ui| {
                    ui.with_layout(egui::Layout::top_down(Align::Center), |ui| {
                        ui.set_width(content_width);
                        ui.heading(format!("Semana {}: Elige nivel", app.quiz.weeks[week_idx].number));
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

                        for (li, level_num, unlocked, completed, new_count) in &level_infos {
                            let label = if *completed && *new_count == 0 {
                                format!("Nivel {} ✅", level_num)
                            } else if *unlocked {
                                if *new_count > 0 {
                                    format!("Nivel {} 🔓 ({} nuevas)", level_num, new_count)
                                } else {
                                    format!("Nivel {} 🔓", level_num)
                                }
                            } else {
                                format!("Nivel {} 🔒", level_num)
                            };

                            ui.horizontal(|ui| {
                                if !*completed {
                                    if ui
                                        .add_enabled(*unlocked, Button::new(&label)
                                            .min_size(Vec2::new(content_width, button_h)))
                                        .clicked()
                                        && *unlocked
                                    {
                                        // Seleccionar nivel y entrar al quiz
                                        app.select_level(week_idx, *li);
                                        app.state = crate::model::AppState::Quiz;
                                        app.update_input_prefill();
                                    }
                                } else {
                                    // Acceso reducido si ya completado
                                    if ui
                                        .add_enabled(*unlocked, Button::new(&label)
                                            .min_size(Vec2::new(content_width * 0.75, button_h)))
                                        .clicked()
                                        && *unlocked
                                    {
                                        app.select_level(week_idx, *li);
                                        app.state = crate::model::AppState::Quiz;
                                        app.update_input_prefill();
                                    }
                                    // Reiniciar nivel
                                    if ui
                                        .add_sized([content_width * 0.21, button_h],
                                                   Button::new("↩").fill(egui::Color32::DARK_RED))
                                        .on_hover_text("Reinicia el nivel")
                                        .clicked()
                                    {
                                        app.reiniciar_nivel(week_idx, *li);
                                        app.select_level(week_idx, *li);
                                        app.state = crate::model::AppState::Quiz;
                                        return;
                                    }
                                }
                            });
                            ui.add_space(8.0);
                        }

                        ui.add_space(16.0);
                        if ui
                            .add_sized([content_width, button_h], Button::new("Volver a semanas"))
                            .clicked()
                        {
                            app.open_week_menu();
                        }
                    });
                });
        });

        ui.add_space(vertical_space / 2.0);
    });
}
