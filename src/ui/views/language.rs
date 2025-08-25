use egui::{Align, Button, CentralPanel, Context, RichText};
use crate::model::{AppState, Language};
use crate::QuizApp;
use crate::update::check_latest_release;

pub fn ui_language_select(app: &mut QuizApp, ctx: &Context) {
    // Limpiamos cualquier mensaje previo
    app.message.clear();

    CentralPanel::default().show(ctx, |ui| {
        // 1) Vertical centering aproximado
        let total_height = 300.0;
        let extra_space = (ui.available_height() - total_height).max(0.0) / 2.0;
        ui.add_space(extra_space);

        // 2) Máximo ancho de contenido
        let max_width = 540.0;
        let content_width = ui.available_width().min(max_width);

        // 3) Layout centrado de arriba a abajo
        ui.with_layout(egui::Layout::top_down(Align::Center), |ui| {
            // panel con borde/márgenes
            egui::Frame::default()
                .fill(ui.visuals().window_fill())
                .inner_margin(egui::Margin::symmetric(16, 16))
                .show(ui, |ui| {
                    ui.set_width(content_width);

                    ui.heading("👋 ¡Bienvenido a summer_quiz!");
                    ui.add_space(18.0);
                    ui.label("Selecciona un lenguaje");
                    ui.add_space(18.0);

                    let button_width = ((content_width - 40.0) / 2.0).clamp(120.0, 280.0);

                    ui.vertical_centered(|ui| {
                        // Botones de lenguaje
                        let btn_c        = ui.add_sized([button_width, 40.0], Button::new("Lenguaje C"));
                        ui.add_space(5.0);

                        let btn_pseudocode = ui.add_sized([button_width, 40.0], Button::new("Pseudocódigo"));
                        ui.add_space(5.0);
                        
                        #[cfg(not(target_arch = "wasm32"))]
                        let btn_exit     = ui.add_sized([button_width, 40.0], Button::new("Salir"));

                        // Al hacer click, actualizamos estado en QuizApp
                        if btn_c.clicked() {
                            app.seleccionar_lenguaje(Language::C);
                        }
                        if btn_pseudocode.clicked() {
                            app.seleccionar_lenguaje(Language::Pseudocode);
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        if btn_exit.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });

                    ui.add_space(16.0);

                    // Comprobamos si hay una actualización pendiente
                    if app.has_update.is_none() {
                        app.has_update = match check_latest_release() {
                            Ok(Some(new_ver)) => Some(new_ver),
                            _                 => Some(String::new()),
                        };
                    }
                    if let Some(ver) = &app.has_update {
                        if !ver.is_empty() {
                            let update_btn = ui.add_sized([button_width, 40.0], Button::new(format!("⬇ Actualizar a {ver}")));
                            if update_btn.clicked() {
                                app.message = "Iniciando actualización…".to_string();
                                app.state = AppState::PendingUpdate;
                                ctx.request_repaint();
                            }
                            ui.add_space(10.0);
                        }
                    }

                    // Mensaje de error / info
                    ui.add_space(12.0);
                    if !app.message.is_empty() {
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(&app.message)
                                .color(egui::Color32::YELLOW)
                                .heading()
                                .strong(),
                        );
                    }
                });
        });
    });
}