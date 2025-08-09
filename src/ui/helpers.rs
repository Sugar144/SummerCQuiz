// src/ui/helpers.rs
use egui::{Ui, Button, Vec2, Color32};

pub fn big_list_button(ui: &mut Ui, label: &str, width: f32, height: f32, enabled: bool) -> bool {
    ui.add_enabled(enabled, Button::new(label).min_size(Vec2::new(width, height))).clicked()
}

pub fn split_button_with_restart(
    ui: &mut Ui,
    label: &str,
    main_w: f32,
    h: f32,
    show_restart: bool,
) -> (bool, bool) {
    let mut clicked_main = false;
    let mut clicked_restart = false;

    ui.horizontal(|ui| {
        clicked_main = ui
            .add_sized([main_w * if show_restart { 0.75 } else { 1.0 }, h], Button::new(label))
            .clicked();

        if show_restart {
            clicked_restart = ui
                .add_sized([main_w * 0.21, h], Button::new("↩").fill(Color32::DARK_RED))
                .on_hover_text("Reiniciar")
                .clicked();
        }
    });

    (clicked_main, clicked_restart)
}
