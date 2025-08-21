// src/ui/helpers.rs
use egui::{Ui, Button, Vec2, Color32};

pub fn big_list_button(ui: &mut Ui, label: String, width: f32, height: f32, enabled: bool) -> bool {
    ui.add_enabled(enabled, Button::new(label).min_size(Vec2::new(width, height))).clicked()
}

/// Devuelve (clicked_main, clicked_restart).
/// - Si `is_completed == false`: SOLO se muestra el botón principal (habilitado).
/// - Si `is_completed == true`: se muestran dos botones:
///     - Principal deshabilitado (con candado y tooltip)
///     - "Reiniciar" habilitado
pub fn split_button_with_restart(
    ui: &mut Ui,
    label: &str,
    total_width: f32,
    height: f32,
    is_completed: bool,
) -> (bool, bool) {
    // Caso 1: NO está completada -> un único botón principal a ancho completo
    if !is_completed {
        let clicked = ui
            .add_sized([total_width, height], Button::new(label))
            .clicked();
        return (clicked, false);
    }

    // Caso 2: SÍ está completada -> principal (bloqueado) + Reiniciar (activo)
    let gap = 8.0;
    let restart_w = (total_width / 4.0).max(80.0); // ancho mínimo sensato
    let main_w = (total_width - restart_w - gap).max(120.0);

    let mut clicked_restart = false;

    ui.horizontal(|ui| {
        // Principal DESHABILITADO con candado y tooltip
        let main_btn = Button::new(format!("{label}  🔒")).min_size(Vec2::new(main_w, height));
        ui.add_enabled(false, main_btn)
            .on_hover_text("Completado: pulsa Reiniciar para volver a intentarlo");

        // Botón Reiniciar ACTIVO
        let restart_btn = Button::new("⟲ Reiniciar")
            .min_size(Vec2::new(restart_w, height))
            .fill(Color32::DARK_RED); // opcional, solo para destacar
        if ui.add(restart_btn).clicked() {
            clicked_restart = true;
        }
    });

    (false, clicked_restart)
}
