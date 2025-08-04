
use egui::{CentralPanel, Context, Ui, Visuals, Frame, Button, ScrollArea};
use egui_code_editor::{CodeEditor, ColorTheme, Syntax};
use crate::QuizApp;

pub fn top_panel(app: &mut QuizApp, ctx: &Context, borrar: bool) {
    egui::TopBottomPanel::top("menu_panel").show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            if borrar {
                if ui.button("🔄 Borrar progreso y reiniciar").clicked() {
                    app.confirm_reset = true;
                    app.has_saved_progress = false;
                }
            }

            if ui.button("Cambiar lenguaje").clicked() {
                app.cambiar_lenguaje();
                ctx.request_repaint();
            }


        });
    });
}

pub fn bottom_panel(ctx: &Context) {
    egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
        // ----------- BOTONES DE TEMA -----------
        ui.with_layout(
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                if ui.button("🌙 Modo oscuro").clicked() {
                    ctx.set_visuals(Visuals::dark());
                }
                if ui.button("☀Modo claro").clicked() {
                    ctx.set_visuals(Visuals::light());
                }
            }
        );

    });
}

/// Panel centrado tanto vertical como horizontalmente,
/// con un tamaño de contenido máximo y un bloque interior `inner`.
pub fn centered_panel(
    ctx: &Context,
    est_height: f32,
    max_width: f32,
    inner: impl FnOnce(&mut Ui),
) {
    CentralPanel::default().show(ctx, |ui| {
        // Espacio vertical para centrar
        let extra = ((ui.available_height() - est_height) / 2.0).max(0.0);
        ui.add_space(extra);
        Frame::default()
            .fill(ui.visuals().window_fill())
            .inner_margin(egui::Margin::symmetric(16, 16))
            .show(ui, |ui| {
                // Ajusta anchura
                let w = ui.available_width().min(max_width);
                ui.set_width(w);
                // Ejecuta contenido
                inner(ui);
            });
        ui.add_space(extra);
    });
}


pub fn simple_panel(
    ctx: &Context,
    max_width: f32,
    margin: egui::Margin,
    inner: impl FnOnce(&mut Ui),
) {
    CentralPanel::default().show(ctx, |ui| {
        let w = ui.available_width().min(max_width);
        Frame::default()
            .fill(ui.visuals().window_fill())
            .inner_margin(margin)
            .show(ui, |ui| {
                ui.set_width(w);
                inner(ui);
            });
    });
}

/// Muestra un editor de solo lectura (solución).
/// Editor de entrada con ancho fijo
pub fn code_editor_input(
    ui: &mut Ui,
    id: &str,
    width: f32,
    rows: usize,
    fontsize: f32,
    syntax: Syntax,
    text: &mut String,
    max_height: f32,
) {
    ScrollArea::vertical()
        .max_height(max_height)
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.set_width(width);
            CodeEditor::default()
                .id_source(id)
                .with_rows(rows)
                .with_fontsize(fontsize)
                .with_theme(ColorTheme::GITHUB_DARK)
                .with_syntax(syntax)
                .with_numlines(true)
                .vscroll(false)
                .show(ui, text);
        });
}

/// Editor de sólo lectura (solución) con ancho fijo
pub fn code_editor_solution(
    ui: &mut Ui,
    width: f32,
    rows: usize,
    fontsize: f32,
    syntax: Syntax,
    code: &str,
    max_height: f32,
) {
    let mut buf = code.to_owned();
    ScrollArea::vertical()
        .max_height(max_height)
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.set_width(width);
            CodeEditor::default()
                .id_source("solution")
                .with_rows(rows)
                .with_fontsize(fontsize)
                .with_theme(ColorTheme::GITHUB_DARK)
                .with_syntax(syntax)
                .with_numlines(true)
                .vscroll(false)
                .show(ui, &mut buf);
        });
}

/// Dibuja dos botones del mismo tamaño en una fila, centrados en el ancho dado.
/// Devuelve (clic izquierdo, clic derecho).
pub fn two_button_row(
    ui: &mut Ui,
    panel_width: f32,
    left_label: &str,
    right_label: &str,
) -> (bool, bool) {
    let btn_w = (panel_width - 8.0) / 2.0;
    let mut clicked_left = false;
    let mut clicked_right = false;
    ui.horizontal(|ui| {
        // espacio para centrar la fila en su panel
        ui.add_space((ui.available_width() - panel_width) / 2.0);
        clicked_left = ui
            .add_sized([btn_w, 36.0], Button::new(left_label))
            .clicked();
        clicked_right = ui
            .add_sized([btn_w, 36.0], Button::new(right_label))
            .clicked();
    });
    (clicked_left, clicked_right)
}


