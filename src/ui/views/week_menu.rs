use egui::{Align, CentralPanel, Context};
use crate::QuizApp;
use crate::ui::helpers::{big_list_button, split_button_with_restart};
use crate::view_models::WeekInfo;

pub fn ui_week_menu(app: &mut QuizApp, ctx: &Context) {
    CentralPanel::default().show(ctx, |ui| {
        let max_width = 400.0;
        let content_width = ui.available_width().min(max_width);
        let button_h = 40.0;

        let infos: Vec<WeekInfo> = app.week_infos();

        // centrar…
        let weeks_count = infos.len() as f32;
        let estimated_h = 80.0 + (button_h + 8.0) * (weeks_count + 1.0);
        let vertical_space = ((ui.available_height() - estimated_h) / 2.0).max(0.0);
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

                        for info in &infos {
                            let label = info.label();

                            let (clicked_main, clicked_restart) =
                                split_button_with_restart(ui, &label, content_width, button_h, info.completed);

                            if clicked_main && info.unlocked {
                                app.progress_mut().current_week = Some(info.idx);
                                app.open_level_menu();
                            }
                            if clicked_restart && info.completed{
                                app.reiniciar_semana(info.idx);
                                app.progress_mut().current_week = Some(info.idx);
                                app.open_level_menu();
                                return;
                            }

                            ui.add_space(5.0);
                        }

                        ui.add_space(10.0);
                        if big_list_button(ui, "Volver al menú principal".to_string(), content_width, button_h, true) {
                            app.volver_al_menu_principal();
                        }
                    });
                });
        });

        ui.add_space(vertical_space / 2.0);
    });
}
