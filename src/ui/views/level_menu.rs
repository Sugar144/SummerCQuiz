use egui::{Align, Button, CentralPanel, Context};
use crate::app::LevelEntry;
use crate::QuizApp;
use crate::view_models::LevelInfo;
use crate::ui::helpers::split_button_with_restart;

pub fn ui_level_menu(app: &mut QuizApp, ctx: &Context) {
    CentralPanel::default().show(ctx, |ui| {
        let max_width = 400.0;
        let content_width = ui.available_width().min(max_width);
        let button_h = 36.0;

        let week_idx = match app.progress().current_week { Some(w) => w, None => return };

        let infos: Vec<LevelInfo> = match app.level_infos_in_current_week() {
            Some(v) => v,
            None => return,
        };

        let estimated_h = 80.0 + (button_h + 8.0) * (infos.len() as f32 + 1.0);
        let vertical_space = ((ui.available_height() - estimated_h) / 2.0).max(0.0);
        ui.add_space(vertical_space / 2.0);

        ui.vertical_centered_justified(|ui| {
            egui::Frame::default()
                .fill(ui.visuals().window_fill())
                .inner_margin(egui::Margin::symmetric(24, 16))
                .show(ui, |ui| {
                    ui.with_layout(egui::Layout::top_down(Align::Center), |ui| {
                        ui.set_width(content_width);
                        ui.heading(format!("Semana {}: Elige nivel", app.quiz.weeks[week_idx].number));
                        ui.add_space(20.0);

                        for info in &infos {
                            let label = info.label();
                            let (clicked_main, clicked_restart) =
                                split_button_with_restart(ui, &label, content_width, button_h, info.completed);

                            if clicked_main && info.unlocked {
                                app.select_level_with_origin(week_idx, info.idx, LevelEntry::Menu);
                                return;
                            }
                            if clicked_restart && info.completed {
                                app.reiniciar_nivel(week_idx, info.idx);
                                app.select_level_with_origin(week_idx, info.idx, LevelEntry::Restart);
                                return;
                            }
                            ui.add_space(8.0);
                        }

                        ui.add_space(16.0);
                        if ui.add_sized([content_width, button_h], Button::new("Volver a semanas")).clicked() {
                            app.open_week_menu();
                        }
                    });
                });
        });

        ui.add_space(vertical_space / 2.0);
    });
}
