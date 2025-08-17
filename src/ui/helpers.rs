// src/ui/helpers.rs
use egui::{Ui, Button, Vec2, Color32};

pub fn big_list_button(ui: &mut Ui, label: &str, width: f32, height: f32, enabled: bool) -> bool {
    ui.add_enabled(enabled, Button::new(label).min_size(Vec2::new(width, height))).clicked()
}

/// Devuelve (clicked_main, clicked_restart).
/// - Si `is_completed` es true, el botón principal aparece deshabilitado (bloqueado)
///   y solo queda activo el botón de Reiniciar.
pub fn split_button_with_restart(
    ui: &mut Ui,
    label: &str,
    total_width: f32,
    height: f32,
    is_completed: bool,
) -> (bool, bool) {
    let gap = 8.0;
    let restart_w = total_width / 4.0; // ancho del botón "Reiniciar"
    let main_w = (total_width - restart_w - gap).max(120.0);

    let mut clicked_main = false;
    let mut clicked_restart = false;

    ui.horizontal(|ui| {
        // Botón principal (bloqueado si completado)
        let main_btn = Button::new(if is_completed {
            format!("{}  🔒", label)
        } else {
            label.to_owned()
        });

        let resp_main = ui.add_sized([main_w, height], main_btn).on_disabled_hover_text("Completado: reinicia para volver a intentarlo");

        if !is_completed && resp_main.clicked() {
            clicked_main = true;
        }

        // Botón Reiniciar (siempre activo cuando is_completed == true; opcionalmente también cuando no)
        let restart_btn = Button::new("⟲").fill(Color32::DARK_RED).selected(true);
        let resp_restart = if is_completed {
            ui.add_sized([restart_w, height], restart_btn)
        } else {
            // Si quieres permitir reiniciar aunque no esté terminado, deja esto igual.
            // Si NO quieres, deshabilítalo cuando !is_completed:
            ui.add_sized([restart_w, height], restart_btn)
            // .on_disabled_hover_text("Aún no has completado este ítem")
        };
        if resp_restart.clicked() {
            clicked_restart = true;
        }
    });

    (clicked_main, clicked_restart)
}
