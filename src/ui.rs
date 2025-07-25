use egui::{Context, Visuals};

use crate::app::QuizApp;
use crate::model::{AppState, Language};
use crate::update::{check_latest_release, descargar_binario_nuevo};

use eframe::{set_value, Frame, APP_KEY};

use egui_code_editor::{CodeEditor, ColorTheme, Syntax};


// ===== SOLO PARA WEB =====
pub fn c_syntax() -> Syntax {
    Syntax::new("c")
        .with_comment("//")
        .with_comment_multiline(["/*", "*/"])
        .with_keywords([
            "int", "char", "void", "if", "else", "for", "while", "return", "break", "continue",
            "switch", "case", "default", "struct", "typedef", "enum", "union", "sizeof", "do",
            "goto", "static", "const", "volatile", "unsigned", "signed", "short", "long", "float",
            "double", "auto", "extern", "register",
        ])
        .with_types([
            "int", "char", "float", "double", "void", "size_t", "uint8_t", "uint16_t", "uint32_t", "uint64_t",
        ])
}



impl eframe::App for QuizApp {

    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        // BOTÓN SUPERIOR DE REINICIAR (solo visible durante el quiz y resumen)
        if matches!(self.state, AppState::Quiz | AppState::Summary) {
            egui::TopBottomPanel::top("menu_panel").show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if ui.button("🔄 Borrar progreso y reiniciar").clicked() {
                        self.confirm_reset = true;
                        self.has_saved_progress = false;
                    }

                    if ui.button("Cambiar lenguaje").clicked() {
                        self.cambiar_lenguaje();
                        ctx.request_repaint();
                    }


                });
            });
        }else if matches!(self.state, AppState::Quiz | AppState::Summary | AppState::Welcome) {
            egui::TopBottomPanel::top("menu_panel").show(ctx, |ui| {
                ui.horizontal_centered(|ui| {

                    if ui.button("Cambiar lenguaje").clicked() {
                        self.cambiar_lenguaje();
                    }
                });
            });
        }

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



        match self.state {
            AppState::PendingUpdate => {
                // Parel central
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(60.0);
                        ui.label(
                            egui::RichText::new(&self.message)
                                .heading()
                                .color(egui::Color32::YELLOW)
                                .strong()
                        );
                        ui.add_space(20.0);
                        ui.add(egui::Spinner::new());
                        ui.add_space(60.0);
                    });
                });

                // Lanzas el thread SOLO la primera vez
                if !self.update_thread_launched {
                    self.update_thread_launched = true;
                    let updater = if cfg!(windows) {
                        "SummerQuizUpdater.exe".to_string()
                    } else {
                        "./SummerQuizUpdater".to_string()
                    };
                    std::thread::spawn(move || {
                        let res = descargar_binario_nuevo();
                        match res {
                            Ok(()) => {
                                std::thread::sleep(std::time::Duration::from_secs(2));
                                std::process::Command::new(&updater)
                                    .spawn()
                                    .expect("No se pudo lanzar el updater");
                                std::process::exit(0);
                            }
                            Err(e) => {
                                eprintln!("Error al descargar actualización: {e}");
                            }
                        }
                    });
                }
                // No se hace nada más, la UI ya ha cambiado el mensaje
            }
            // ----------- BIENVENIDA -----------
            AppState::LanguageSelect => {
                self.message.clear();
                egui::CentralPanel::default().show(ctx, |ui| {
                    // Calcula el espacio vertical extra
                    let total_height = 300.0; // tu contenido: aprox. heading + botones, etc.
                    let extra_space = (ui.available_height() - total_height).max(0.0) / 2.0;
                    ui.add_space(extra_space);

                    // Máximo ancho que quieres permitir (por si pantalla ultra-wide)
                    let max_width = 540.0;
                    let content_width = ui.available_width().min(max_width);

                    // Centrar
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        // Opcional: añade un panel contenedor para poner un borde, margen, color, etc.
                        egui::Frame::default()
                            .fill(ui.visuals().window_fill())
                            .inner_margin(egui::Margin::symmetric(16, 16))
                            .show(ui, |ui| {
                                ui.set_width(content_width);

                                ui.heading("👋 ¡Bienvenido a SummerQuiz!");
                                ui.add_space(18.0);
                                ui.label("Selecciona un lenguaje");
                                ui.add_space(18.0);

                                let button_width = (content_width - 40.0) / 2.0;
                                let button_width = button_width.clamp(120.0, 280.0); // Nunca demasiado pequeño ni enorme

                                // Botones centrados y adaptativos
                                ui.vertical_centered(|ui| {

                                    let c = ui.add_sized([button_width, 40.0], egui::Button::new("Lenguaje C"));
                                    let pseudocode = ui.add_sized([button_width, 40.0], egui::Button::new("Pseudocódigo"));
                                    #[cfg (not(target_arch = "wasm32"))]
                                    let salir = ui.add_sized([button_width, 40.0], egui::Button::new("Salir"));


                                    let mut selected: Option<Language> = None;
                                    if c.clicked() {
                                        selected = Some(Language::C);
                                    }
                                    if pseudocode.clicked() {
                                        selected = Some(Language::Pseudocode);
                                    }
                                    if let Some(lang) = selected {
                                        self.seleccionar_lenguaje(lang);
                                    }

                                    #[cfg (not(target_arch = "wasm32"))]
                                    if salir.clicked() {
                                        std::process::exit(0);
                                    }


                                });

                                ui.add_space(16.0);

                                if self.has_update.is_none() {
                                    self.has_update = match check_latest_release() {
                                        Ok(Some(new_ver)) => Some(new_ver),
                                        Ok(None) => Some("".to_string()),
                                        Err(_) => Some("".to_string()),
                                    };
                                }

                                if let Some(ver) = &self.has_update {
                                    if !ver.is_empty() {
                                        let update = ui.add_sized(
                                            [button_width, 40.0],
                                            egui::Button::new(format!("⬇ Actualizar a {ver}"))
                                        );

                                        if update.clicked() {
                                            self.message = "Iniciando actualización…".to_string();
                                            self.state = AppState::PendingUpdate; // Cambia el estado
                                            ctx.request_repaint();

                                        }

                                        ui.add_space(10.0);
                                    }
                                }
                                ui.add_space(12.0);

                                if !self.message.is_empty() {
                                    ui.add_space(10.0);
                                    ui.label(
                                        egui::RichText::new(&self.message)
                                            .color(egui::Color32::YELLOW)
                                            .heading()
                                            .strong()
                                    );
                                }
                            });
                    });
                });
            }



            // ----------- BIENVENIDA -----------
            AppState::Welcome => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let max_width = 540.0;
                    let available_w = ui.available_width();
                    let content_width = available_w.min(max_width);

                    // Estima altura del bloque de contenido
                    let estimated_height = 230.0;
                    let vertical_space = ((ui.available_height() - estimated_height) / 2.0).max(0.0);

                    ui.add_space(vertical_space / 2.0);

                    ui.horizontal_centered(|ui| {
                        egui::Frame::default()
                            .fill(ui.visuals().window_fill())
                            .inner_margin(egui::Margin::symmetric(16, 16))
                            .show(ui, |ui| {
                                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                    ui.heading("¿Qué deseas hacer?");
                                    ui.add_space(18.0);

                                    let hay_guardado = self.has_saved_progress;
                                    let button_w = (content_width * 0.9).clamp(120.0, 400.0);
                                    let button_h = 36.0;

                                    let continuar_btn = if hay_guardado {
                                        Some(ui.add_sized([button_w, button_h], egui::Button::new("▶ Continuar donde lo dejé")))
                                    } else {
                                        None
                                    };

                                    let empezar_btn = ui.add_sized([button_w, button_h], egui::Button::new("🔄 Empezar de 0"));

                                    let menu_semanal_btn = ui.add_sized([button_w, button_h], egui::Button::new("📅 Seleccionar Semana"));

                                    let salir_btn = ui.add_sized([button_w, button_h], egui::Button::new("🔙 Volver"));


                                    // --- Manejar clicks ---
                                    if let Some(btn) = continuar_btn {
                                        if btn.clicked() {
                                            self.continuar_quiz();
                                        }
                                    }

                                    if empezar_btn.clicked() && hay_guardado {
                                        self.confirm_reset = true;
                                    } else if empezar_btn.clicked() {
                                        self.empezar_desde_cero();
                                    }

                                    if menu_semanal_btn.clicked() {
                                        self.abrir_menu_semanal();                                    }

                                    if salir_btn.clicked() {
                                        self.salir_app();                                    }

                                    if self.confirm_reset {
                                        self.confirm_reset(ctx);
                                    }

                                    if self.hay_preguntas_nuevas() {
                                        ui.label(
                                            egui::RichText::new("🟡 ¡Nuevas preguntas disponibles! Revisa las semanas completadas.")
                                                .color(egui::Color32::YELLOW)
                                                .heading()
                                                .strong()
                                        );
                                    }

                                });
                            });
                    });

                    ui.add_space(vertical_space);
                });
            }

            AppState::WeekMenu => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let max_width = 540.0;
                    let content_width = ui.available_width().min(max_width);
                    let button_w = (content_width * 0.9).clamp(140.0, 400.0);
                    let button_h = 36.0;

                    // Calcular nº de semanas para estimar altura
                    let weeks_count = self.questions
                        .iter()
                        .filter(|q| q.language == self.selected_language.unwrap_or(Language::C))
                        .map(|q| q.week)
                        .collect::<std::collections::HashSet<_>>()
                        .len();

                    let estimated_height = 80 + (button_h as usize + 8) * (weeks_count + 1);
                    let vertical_space = ((ui.available_height() - estimated_height as f32) / 2.0).max(0.0);

                    ui.add_space(vertical_space / 2.0);

                    // Este bloque centra el Frame horizontalmente
                    ui.horizontal_centered(|ui| {
                        egui::Frame::default()
                            .fill(ui.visuals().window_fill())
                            .inner_margin(egui::Margin::symmetric(24, 16))
                            .show(ui, |ui| {
                                // No pongas set_width aquí
                                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                    ui.heading("Selecciona una semana");
                                    ui.add_space(20.0);

                                    let language = self.selected_language.unwrap_or(Language::C);

                                    let mut weeks_with_questions: Vec<usize> = self.questions
                                        .iter()
                                        .filter(|q| q.language == language)
                                        .map(|q| q.week)
                                        .collect();

                                    weeks_with_questions.sort_unstable();
                                    weeks_with_questions.dedup();

                                    let mut buttons = vec![];

                                    for &week in &weeks_with_questions {
                                        let unlocked = self.is_week_unlocked(week);
                                        let completed = self.is_week_completed(week);
                                        let n_nuevas = self.nuevas_preguntas_en_semana(week, language);

                                        let label = if completed && n_nuevas == 0 {
                                            format!("Semana {} ✅", week)
                                        } else if unlocked {
                                            // Aquí puedes mostrar si hay nuevas preguntas aunque estuviera completada antes
                                            if n_nuevas > 0 {
                                                format!("Semana {} 🔓 ({} nuevas)", week, n_nuevas)
                                            } else {
                                                format!("Semana {} 🔓", week)
                                            }
                                        } else {
                                            format!("Semana {} 🔒", week)
                                        };

                                        let button = ui.add_sized(
                                            [button_w, button_h],
                                            egui::Button::new(label)
                                        ).on_hover_text("Pulsa para acceder a esta semana");


                                        // --- Botón reiniciar solo si la semana está completada
                                        if completed {
                                            ui.add_space(2.0);
                                            if ui.button(format!("🔄")).clicked() {
                                                self.reiniciar_semana(week);
                                                self.acceder_a_semana(week); // Entra directamente
                                                return; // Sale del bucle para evitar clicks dobles
                                            }
                                        }


                                        buttons.push((week, button, unlocked));
                                        ui.add_space(4.0);
                                    }

                                    ui.add_space(16.0);

                                    let volver_btn = ui.add_sized([button_w, button_h], egui::Button::new("Volver al menú principal"));

                                    // --- Gestión de clicks ---
                                    for (week, button, unlocked) in buttons {
                                        if button.clicked() && unlocked {
                                            self.acceder_a_semana(week);
                                        }
                                    }
                                    if volver_btn.clicked() {
                                        self.volver_al_menu_principal();
                                    }
                                });
                            });
                    });

                    ui.add_space(vertical_space);
                });
            }





            // ----------- QUIZ NORMAL -----------
            AppState::Quiz => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let max_width = 600.0;
                    let panel_width = (ui.available_width() * 0.97).min(max_width);
                    let total_height = 150.0 + 245.0 + 48.0 + 48.0 + 24.0;
                    let extra_space = (ui.available_height() - total_height).max(0.0) / 2.0;
                    ui.add_space(extra_space);

                    egui::Frame::default()
                        .fill(ui.visuals().window_fill())
                        .inner_margin(egui::Margin::symmetric(120, 20))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                if let Some(idx) = self.current_in_week {
                                    ui.heading(format!("🌀 Ronda {}", self.round));
                                    // Prompt con scroll fijo
                                    let prompt_max_height = 250.0;
                                    let prompt_min_lines = 4.0;
                                    let font_id = egui::TextStyle::Body.resolve(ui.style());
                                    let line_height = ui.fonts(|f| f.row_height(&font_id));
                                    let prompt_min_height = prompt_min_lines * line_height;
                                    let prompt_text = self.questions[idx].prompt.clone();
                                    let galley = ui.fonts(|fonts| {
                                        fonts.layout(
                                            prompt_text.to_owned(),
                                            font_id.clone(),
                                            egui::Color32::WHITE,
                                            panel_width,
                                        )
                                    });
                                    let needed_height = galley.size().y.max(prompt_min_height).min(prompt_max_height);
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(panel_width, needed_height),
                                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                                        |ui| {
                                            egui::ScrollArea::vertical()
                                                .max_height(prompt_max_height)
                                                .show(ui, |ui| {
                                                    ui.with_layout(
                                                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                                                        |ui| {
                                                            ui.label(&prompt_text);
                                                        }
                                                    );
                                                });
                                        }
                                    );

                                    ui.add_space(5.0);

                                    let max_input_height = 245.0;
                                    let min_lines = 15;
                                    let font_id = egui::TextStyle::Monospace.resolve(ui.style());
                                    let line_height = ui.fonts(|f| f.row_height(&font_id));
                                    let code_rows = min_lines;

                                    if self.questions[idx].fails >= 2 {
                                        // Si no se ha mostrado la solución todavía
                                        if !self.show_solution {
                                            if ui.button("Solución").clicked() {
                                                self.show_solution = true;
                                            }

                                            // INPUT DEL USUARIO con resaltado (editable)
                                            egui::ScrollArea::vertical()
                                                .max_height(max_input_height)
                                                .auto_shrink([false; 2])
                                                .show(ui, |ui| {
                                                    ui.set_width(panel_width);
                                                    CodeEditor::default()
                                                        .id_source("user_input")
                                                        .with_rows(code_rows)
                                                        .with_fontsize(line_height)
                                                        .with_theme(ColorTheme::GITHUB_DARK)
                                                        .with_syntax(c_syntax())
                                                        .with_numlines(true)
                                                        .vscroll(false)
                                                        .show(ui, &mut self.input);
                                                });
                                        } else {
                                            if ui.button("Siguiente pregunta").clicked() {
                                                self.avanzar_a_siguiente_pregunta(idx);
                                            }

                                            // SOLUCIÓN con resaltado (editable)
                                            let mut answer_string = self.questions[idx].answer.clone();
                                            egui::ScrollArea::vertical()
                                                .max_height(max_input_height)
                                                .auto_shrink([false; 2])
                                                .show(ui, |ui| {
                                                    ui.set_width(panel_width);

                                                    CodeEditor::default()
                                                        .id_source("solution")
                                                        .with_rows(code_rows)
                                                        .with_fontsize(line_height)
                                                        .with_theme(ColorTheme::GITHUB_DARK)
                                                        .with_syntax(c_syntax())
                                                        .with_numlines(true)
                                                        .vscroll(false)
                                                        .show(ui, &mut answer_string);
                                                });
                                        }
                                    } else {
                                        // INPUT DEL USUARIO con resaltado (editable)
                                        egui::ScrollArea::vertical()
                                            .max_height(max_input_height)
                                            .auto_shrink([false; 2])
                                            .show(ui, |ui| {
                                                ui.set_width(panel_width);

                                                CodeEditor::default()
                                                    .id_source("user_input")
                                                    .with_rows(code_rows)
                                                    .with_fontsize(line_height)
                                                    .with_theme(ColorTheme::GITHUB_DARK)
                                                    .with_syntax(c_syntax())
                                                    .with_numlines(true)
                                                    .vscroll(false)
                                                    .show(ui, &mut self.input);
                                            });
                                    }

                                    if self.questions[idx].fails >= 1 {
                                        if let Some(hint) = &self.questions[idx].hint {
                                            ui.label(format!("💡 Pista: {hint}"));
                                        }
                                    }

                                    ui.add_space(5.0);

                                    if ui.button("⚡ Marcar semana como completada (TEST)").clicked() {
                                        let week = self.current_week.unwrap_or(1);
                                        let language = self.selected_language.unwrap_or(Language::C);
                                        for q in self.questions.iter_mut() {
                                            if q.week == week && q.language == language {
                                                q.is_done = true;
                                                q.saw_solution = false;
                                                q.attempts = 1;
                                                q.fails = 0;
                                                q.skips = 0;
                                                if let Some(id) = &q.id {
                                                    self.completed_ids.insert(id.clone());
                                                }
                                            }
                                        }
                                        self.current_in_week = self.next_pending_in_week();
                                        // Si ya no quedan preguntas, muestra resumen
                                        if self.current_in_week.is_none() {
                                            self.state = AppState::Summary;
                                        }
                                    }


                                    // Botones
                                    ui.horizontal(|ui| {
                                        ui.add_space((ui.available_width() - panel_width) / 2.0);
                                        let button_width = (panel_width - 8.0) / 2.0;
                                        let enviar = ui.add_sized([button_width, 36.0], egui::Button::new("Enviar"));
                                        let saltar = ui.add_sized([button_width, 36.0], egui::Button::new("Saltar pregunta"));

                                        if enviar.clicked() {
                                            if let Some(idx) = self.current_in_week {
                                                let input = self.input.clone();
                                                self.procesar_respuesta(&input, idx);
                                            }
                                        }
                                        if saltar.clicked() {
                                            self.saltar_pregunta(idx);
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.add_space((ui.available_width() - panel_width) / 2.0);

                                        let button_width = (panel_width - 8.0) / 2.0;
                                        let guardar = ui.add_sized([button_width, 36.0], egui::Button::new("Volver"));
                                        let progreso = ui.add_sized([button_width, 36.0], egui::Button::new("Ver progreso"));

                                        if progreso.clicked() {
                                            self.ver_progreso()
                                        }

                                        if guardar.clicked() {
                                            self.guardar_y_salir();
                                        }
                                    });

                                    ui.add_space(8.0);

                                    if !self.message.is_empty() {
                                        ui.label(&self.message);
                                    }
                                }
                            });
                        });

                    ui.add_space(extra_space);
                });
            }



            // ----------- RESUMEN FINAL -----------
            AppState::Summary => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let max_width = 600.0;
                    let panel_width = (ui.available_width() * 0.97).min(max_width);
                    let button_width = panel_width / 3.0;
                    let button_height = 36.0;
                    let total_height = 700.0;
                    let extra_space = (ui.available_height() - total_height).max(0.0) / 2.0;

                    ui.add_space(extra_space);


                    ui.vertical_centered(|ui| {
                        egui::Frame::default()
                            .fill(ui.visuals().window_fill())
                            .inner_margin(egui::Margin::symmetric(16, 50))
                            .show(ui, |ui| {
                                ui.set_width(panel_width / 1.5);

                                ui.heading("Progreso Actual");
                                ui.add_space(10.0);
                                ui.label("Resumen de preguntas:");
                                ui.add_space(5.0);

                                let max_height = 700.0;

                                egui::ScrollArea::vertical()
                                    .max_height(max_height)
                                    .max_width(panel_width)
                                    .show(ui, |ui| {
                                        let week = self.current_week.unwrap_or(1);

                                        egui::Grid::new("quiz_results_grid")
                                            .striped(true)
                                            .spacing([8.0, 0.0])
                                            .show(ui, |ui| {
                                                ui.label("Pregunta");
                                                ui.label("Intentos");
                                                ui.label("Fallos");
                                                ui.label("Saltos");
                                                ui.label("Solución vista");
                                                ui.label("Estado");
                                                ui.end_row();

                                                for (i, q) in self.questions.iter().enumerate() {
                                                    if q.week != week { continue; }
                                                    let status = if q.is_done && !q.saw_solution {
                                                        "✅ Correcta"
                                                    } else if q.saw_solution {
                                                        "❌ Fallida"
                                                    } else {
                                                        "❌ Sin responder"
                                                    };
                                                    let solucion_vista = if q.saw_solution { "Sí" } else { "No" };
                                                    ui.label(format!("{}", i + 1));
                                                    ui.label(format!("{}", q.attempts));
                                                    ui.label(format!("{}", q.fails));
                                                    ui.label(format!("{}", q.skips));
                                                    ui.label(solucion_vista);
                                                    ui.label(status);
                                                    ui.end_row();
                                                }
                                            });
                                    });


                                ui.add_space(5.0);


                                // Aquí los botones, dentro del mismo bloque
                                ui.vertical_centered(|ui| {

                                    let current_week = self.current_week.unwrap_or(1);
                                    let language = self.selected_language.unwrap_or(Language::C);
                                    let total_weeks = self.questions
                                        .iter()
                                        .filter(|q| q.language == language)
                                        .map(|q| q.week)
                                        .max()
                                        .unwrap_or(current_week);

                                    let is_current_week_complete = self.is_week_completed(current_week);
                                    let has_next_week = current_week < total_weeks;

                                    if is_current_week_complete && has_next_week {
                                        let siguiente = ui.add_sized([button_width, button_height], egui::Button::new("Siguiente Semana"));
                                        if siguiente.clicked() {
                                            self.avanzar_a_siguiente_semana(current_week);
                                        }
                                    } else {
                                        ui.add_space(10.0);
                                        ui.label("¡Bien hecho! Has completado todas las semana disponibles, pulsa volver para ir al menú.");
                                        ui.add_space(10.0);

                                        let volver = ui.add_sized([button_width, button_height], egui::Button::new("Volver"));
                                        if volver.clicked() {
                                            self.guardar_y_salir();
                                        }
                                    }
                                });
                            });
                    });
                });
            }
        }

        if self.confirm_reset {
            self.confirm_reset(ctx);
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Esto guardará el estado automáticamente en web y escritorio
        set_value(storage, APP_KEY, self);
    }
}