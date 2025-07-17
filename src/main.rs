use eframe::egui;
use egui::Visuals;
use serde::{Serialize, Deserialize };

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
enum Language {
    C,
    Pseudocode
}

#[derive(Serialize, Deserialize)]
struct Question {
    language: Language,
    week: usize,
    prompt: String,  // Preguntas
    answer: String,  // Respuestas
    hint:Option<String>,
    #[serde(default)]
    is_done: bool,   // true si respondida correctamente
    #[serde(default)]
    saw_solution: bool,
    #[serde(default)]
    attempts: u32,   // intentos totales (aciertos+fallos+saltos)
    #[serde(default)]
    fails: u32,      // respuestas incorrectas
    #[serde(default)]
    skips: u32,      // veces saltadas
}

enum AppState {
    LanguageSelect,
    Welcome,
    WeekMenu,
    Quiz,
    Summary,
}

// ¡Implementa Default!
impl Default for AppState {
    fn default() -> Self {
        AppState::Welcome
    }
}

#[derive(Serialize, Deserialize)]
struct QuizApp {
    questions: Vec<Question>,
    selected_language: Option<Language>,
    current_week: Option<usize>,
    current_in_week: Option<usize>,
    input: String,
    message: String,
    finished: bool,
    round: u32,
    shown_this_round: Vec<usize>,
    show_solution: bool,
    #[serde(skip)]
    state: AppState,
}

fn progress_filename(language: Language) -> &'static str {
    match language {
        Language::C => "quiz_progress_c.json",
        Language::Pseudocode => "quiz_progress_pseudocode.json",
    }
}

impl QuizApp {
    pub fn new() -> Self {
        let questions = read_questions_embedded();
        Self {
            questions,
            selected_language: None,
            current_week: None,
            current_in_week: None,
            input: String::new(),
            message: String::new(),
            finished: false,
            round: 1,
            shown_this_round: vec![],
            show_solution: false,
            state: AppState::LanguageSelect,
        }
    }

    pub fn new_for_language(language: Language) -> Self {
        let questions = read_questions_embedded()
            .into_iter()
            .filter(|q| q.language == language)
            .collect::<Vec<_>>();
        let first_week = questions.iter().map(|q| q.week).min().unwrap_or(1);
        let current_in_week = questions.iter().position(|q| q.week == first_week && !q.is_done);
        Self {
            questions,
            selected_language: Some(language),
            current_week: Some(first_week),
            current_in_week,
            input: String::new(),
            message: String::new(),
            finished: false,
            round: 1,
            shown_this_round: vec![],
            show_solution: false,
            state: AppState::Welcome,
        }
    }
    pub fn save_progress(&self) {
        if let Some(lang) = self.selected_language {
            let filename = progress_filename(lang);
            let json = serde_json::to_string(self).unwrap();
            std::fs::write(filename, json).unwrap();
        }
    }

    pub fn load_progress(language: Language) -> Option<Self> {
        let filename = progress_filename(language);
        if let Ok(json) = std::fs::read_to_string(filename) {
            serde_json::from_str(&json).ok()
        } else {
            None
        }
    }

    pub fn delete_progress(language: Language) {
        let filename = match language {
            Language::C => "quiz_progress_c.json",
            Language::Pseudocode => "quiz_progress_pseudocode.json",
        };
        let _ = std::fs::remove_file(filename);
    }

    pub fn has_saved_progress(language: Language) -> bool {
        let filename = match language {
            Language::C => "quiz_progress_c.json",
            Language::Pseudocode => "quiz_progress_pseudocode.json",
        };
        std::fs::metadata(filename).is_ok()
    }

    fn select_week(&mut self, week: usize) {
        self.current_week = Some(week);
        let language = self.selected_language.unwrap_or(Language::C);

        // Busca el índice GLOBAL de la primera pregunta pendiente de esa semana y lenguaje
        self.current_in_week = self.questions
            .iter()
            .enumerate()
            .find(|(_, q)| q.week == week && q.language == language && !q.is_done)
            .map(|(idx, _)| idx);
        self.round = 1;
        self.shown_this_round.clear();
    }


    // Una semana está desbloqueada si todas las anteriores están completas, o es la primera
    pub fn is_week_unlocked(&self, week: usize) -> bool {
        if week == 1 { return true; }
        let language = self.selected_language.unwrap_or(Language::C);
        (1..week).all(|w| self.questions
            .iter()
            .filter(|q| q.week == w && q.language == language)
            .all(|q| q.is_done)
        )
    }


    // Una semana está completa si todas sus preguntas están respondidas correctamente
    pub fn is_week_completed(&self, week: usize) -> bool {
        let language = self.selected_language.unwrap_or(Language::C);
        self.questions
            .iter()
            .filter(|q| q.week == week && q.language == language)
            .all(|q| q.is_done)
    }



    /// Busca la próxima pregunta pendiente, o None si no quedan
    fn next_pending_in_week(&mut self) -> Option<usize> {
        if let Some(week) = self.current_week {
            for (idx, q) in self.questions.iter().enumerate() {
                if q.week == week && !q.is_done && !self.shown_this_round.contains(&idx) {
                    self.shown_this_round.push(idx);
                    return Some(idx);
                }
            }
            // Si ya se han mostrado todas las pendientes, empieza nueva ronda
            if self.questions.iter().any(|q| q.week == week && !q.is_done) {
                self.round += 1;
                self.shown_this_round.clear();
                for (idx, q) in self.questions.iter().enumerate() {
                    if q.week == week && !q.is_done {
                        self.shown_this_round.push(idx);
                        return Some(idx);
                    }
                }
            }
        }
        None
    }

}

fn normalize_code(input: &str) -> String {
    input
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.starts_with("//") && !line.is_empty())
        .collect::<Vec<_>>()
        .join("")
        .replace(char::is_whitespace, "")
}


fn read_questions_embedded() -> Vec<Question> {
    // Ruta relativa al archivo fuente donde pongas este código.
    // Si lo tienes en data/, usa así:
    let file_content = include_str!("data/quiz_questions.yaml");
    serde_yaml::from_str(file_content).expect("No se pudo parsear el banco de preguntas YAML")
}



// fn show_highlighted_code(ui: &mut egui::Ui, code: &str, _language: Language) {
//     // ... Aquí tu lógica de resaltado, o simplemente:
//     ui.add(
//         egui::TextEdit::multiline(&mut code.to_string())
//             .desired_rows(10)
//             .font(egui::TextStyle::Monospace)
//             .interactive(false)
//     );
// }


impl eframe::App for QuizApp {

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // BOTÓN SUPERIOR DE REINICIAR (solo visible durante el quiz y resumen)
        if matches!(self.state, AppState::Quiz | AppState::Summary) {
            egui::TopBottomPanel::top("menu_panel").show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if ui.button("🔄 Borrar progreso y reiniciar").clicked() {
                        Self::delete_progress(self.selected_language.unwrap());
                        *self = QuizApp::new();
                    }

                    if ui.button("Cambiar lenguaje").clicked() {
                        self.save_progress();
                        self.state = AppState::LanguageSelect;

                    }

                });
            });
        }else if matches!(self.state, AppState::Quiz | AppState::Summary | AppState::Welcome) {
            egui::TopBottomPanel::top("menu_panel").show(ctx, |ui| {
                ui.horizontal_centered(|ui| {

                    if ui.button("Cambiar lenguaje").clicked() {
                        self.save_progress();
                        self.state = AppState::LanguageSelect;

                    }

                });
            });
        }

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            // ----------- BOTONES DE TEMA -----------
            ui.horizontal(|ui| {
                ui.add_space(595.0);
                if ui.button("🌙 Modo oscuro").clicked() {
                    ctx.set_visuals(Visuals::dark());
                }
                if ui.button("☀Modo claro").clicked() {
                    ctx.set_visuals(Visuals::light());
                }
            });
        });



        match self.state {
            // ----------- BIENVENIDA -----------
            AppState::LanguageSelect => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let max_width = 600.0;
                    let panel_width = (ui.available_width() * 0.97).min(max_width);
                    let button_width = (panel_width - 8.0) / 3.0;
                    let button_height = 36.0;
                    let total_height = 240.0;
                    let extra_space = (ui.available_height() - total_height).max(0.0) / 2.0;
                    ui.add_space(extra_space);

                    egui::Frame::default()
                        .fill(ui.visuals().window_fill())
                        .inner_margin(egui::Margin::symmetric(20, 20))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.heading("👋 ¡Bienvenido a SummerQuiz!");
                                ui.add_space(30.0);

                                ui.label("Selecciona un lenguaje");
                                ui.add_space(10.0);

                                let c = ui.add_sized([button_width, button_height], egui::Button::new("Lenguaje C"));
                                let pseudocode = ui.add_sized([button_width, button_height], egui::Button::new("Pseudocódigo"));

                                if c.clicked() {
                                    self.selected_language = Some(Language::C);
                                    *self = QuizApp::load_progress(Language::C).unwrap_or_else(|| QuizApp::new_for_language(Language::C));
                                    self.state = AppState::Welcome;
                                }

                                if pseudocode.clicked() {
                                    self.selected_language = Some(Language::Pseudocode);
                                    *self = QuizApp::load_progress(Language::Pseudocode).unwrap_or_else(|| QuizApp::new_for_language(Language::Pseudocode));
                                    self.state = AppState::Welcome;
                                }

                            });
                        });

                });
            }


            // ----------- BIENVENIDA -----------
            AppState::Welcome => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let max_width = 600.0;
                    let panel_width = (ui.available_width() * 0.97).min(max_width);
                    let button_width = (panel_width - 8.0) / 3.0;
                    let button_height = 36.0;
                    let total_height = 240.0;
                    let extra_space = (ui.available_height() - total_height).max(0.0) / 2.0;
                    ui.add_space(extra_space);

                    egui::Frame::default()
                        .fill(ui.visuals().window_fill())
                        .inner_margin(egui::Margin::symmetric(20, 20))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {

                                ui.heading("¿Qué deseas hacer?");

                                let hay_guardado = Self::has_saved_progress(self.selected_language.unwrap());

                                ui.vertical_centered(|ui| {
                                    ui.add_space(16.0);

                                    if hay_guardado {
                                        let continuar = ui.add_sized([button_width, button_height], egui::Button::new("▶ Continuar donde lo dejé"));

                                        if continuar.clicked() {
                                            if self.current_week.is_none() || self.current_in_week.is_none() {
                                                if let Some(first_week) = self.questions.iter().filter(|q| !q.is_done).map(|q| q.week).min() {
                                                    self.select_week(first_week);
                                                } else {
                                                    let first_week = self.questions.iter().map(|q| q.week).min().unwrap_or(1);
                                                    self.select_week(first_week);
                                                }
                                            }
                                            self.state = AppState::Quiz;
                                            self.finished = false;
                                            self.input.clear();
                                            self.message.clear();
                                        }
                                    }

                                    let empezar = ui.add_sized([button_width, button_height], egui::Button::new("🔄 Empezar de 0"));
                                    let menu_semanal = ui.add_sized([button_width, button_height], egui::Button::new(" 📅 Seleccionar Semana"));
                                    let salir = ui.add_sized([button_width, button_height], egui::Button::new("❌ Salir"));

                                    if empezar.clicked() {
                                        Self::delete_progress(self.selected_language.unwrap());

                                        *self = QuizApp::new_for_language(self.selected_language.unwrap());
                                        self.state = AppState::Quiz;
                                    }

                                    if menu_semanal.clicked() {
                                        self.state = AppState::WeekMenu;
                                    }

                                    if salir.clicked() {
                                        std::process::exit(0);
                                    }
                                });
                            });
                        });

                    ui.add_space(extra_space);
                });
            }


            AppState::WeekMenu => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    //let max_width = 600.0;
                    let button_width = 320.0; // O el valor que uses para tus otros botones
                    let button_height = 36.0;
                    let total_height = 100.0 + (button_height + 8.0) * (self.questions.iter().map(|q| q.week).max().unwrap_or(1) as f32);

                    // Centrado vertical en la pantalla
                    let extra_space = (ui.available_height() - total_height).max(0.0) / 2.0;
                    ui.add_space(extra_space);

                    // Frame opcional para visual consistente
                    egui::Frame::default()
                        .fill(ui.visuals().window_fill())
                        .inner_margin(egui::Margin::symmetric(40, 20))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.heading("Selecciona una semana");
                                ui.add_space(20.0);

                                let language = self.selected_language.unwrap_or(Language::C);

                                let total_weeks = self.questions
                                    .iter()
                                    .filter(|q| q.language == language)  // <-- Aquí el filtro por lenguaje
                                    .map(|q| q.week)
                                    .max()
                                    .unwrap_or(1);

                                for week in 1..=total_weeks {
                                    let unlocked = self.is_week_unlocked(week);
                                    let completed = self.is_week_completed(week);
                                    let label = if completed {
                                        format!("Semana {} ✅", week)
                                    } else if unlocked {
                                        format!("Semana {} 🔓", week)
                                    } else {
                                        format!("Semana {} 🔒", week)
                                    };

                                    let button = ui.add_sized(
                                        [button_width, button_height],
                                        egui::Button::new(label)
                                    ).on_hover_text("Pulsa para acceder a esta semana");
                                    if button.clicked() && unlocked {
                                        self.select_week(week);
                                        self.state = AppState::Quiz;
                                    }
                                    ui.add_space(8.0);
                                }

                                ui.add_space(24.0);
                                if ui.add_sized([button_width, button_height], egui::Button::new("Volver al menú principal")).clicked() {
                                    self.state = AppState::Welcome;
                                }
                            });
                        });

                    ui.add_space(extra_space);
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
                                    let prompt_text = &self.questions[idx].prompt;
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
                                                            ui.label(prompt_text);
                                                        }
                                                    );
                                                });
                                        }
                                    );



                                    ui.add_space(5.0);

                                    let max_input_height = 245.0;

                                    let answer_box = |ui: &mut egui::Ui, content: &mut String, editable: bool| {
                                        egui::ScrollArea::vertical()
                                            .max_height(max_input_height)
                                            .auto_shrink([false; 2])
                                            .show(ui, |ui| {
                                                let text_edit = egui::TextEdit::multiline(content)
                                                    .desired_width(panel_width)
                                                    .desired_rows(16)
                                                    .font(egui::TextStyle::Monospace)
                                                    .interactive(editable);
                                                ui.add(text_edit);
                                            });
                                    };


                                    if self.questions[idx].fails >= 1 {

                                        // Si no se ha mostrado la solución todavía
                                        if !self.show_solution {
                                            ui.horizontal(|ui| {
                                                if ui.button("Solución").clicked() {
                                                    self.show_solution = true;
                                                }
                                            });
                                            answer_box(ui, &mut self.input, true);
                                        } else {
                                            ui.horizontal(|ui | {
                                                if ui.button("Siguiente pregunta").clicked() {
                                                    self.questions[idx].is_done = true;
                                                    self.questions[idx].saw_solution = true;
                                                    self.show_solution = false; // Reset
                                                    self.input.clear();
                                                    self.current_in_week = self.next_pending_in_week();
                                                    if self.current_in_week.is_none() {
                                                        self.state = AppState::Summary;
                                                    }
                                                    self.save_progress();
                                                }
                                            });

                                            let answer_string = self.questions[idx].answer.clone();
                                            answer_box(ui, &mut answer_string.clone(), false);
                                        }
                                    } else {
                                        answer_box(ui, &mut self.input, true);
                                    }


                                    if self.questions[idx].fails >= 1 {
                                        if let Some(hint) = &self.questions[idx].hint {
                                            ui.label(format!("💡 Pista: {hint}"));
                                        }
                                    }

                                    ui.add_space(5.0);


                                    // Botones
                                    ui.horizontal(|ui| {
                                        ui.add_space((ui.available_width() - panel_width) / 2.0);
                                        let button_width = (panel_width - 8.0) / 2.0;
                                        let enviar = ui.add_sized([button_width, 36.0], egui::Button::new("Enviar"));
                                        let saltar = ui.add_sized([button_width, 36.0], egui::Button::new("Saltar pregunta"));

                                        if enviar.clicked() {
                                            if self.input.trim().is_empty() {
                                                self.message = "⚠ Debes escribir una respuesta antes de enviar.".to_string();
                                            } else {
                                                let user_code = normalize_code(&self.input);
                                                let answer_code = normalize_code(&self.questions[idx].answer);
                                                self.questions[idx].attempts += 1;

                                                // ¡Marca esta pregunta como mostrada en la ronda actual!
                                                if !self.shown_this_round.contains(&idx) {
                                                    self.shown_this_round.push(idx);
                                                }

                                                if user_code == answer_code {
                                                    self.questions[idx].is_done = true;
                                                    self.message = "✅ ¡Correcto!".to_string();
                                                    self.input.clear();
                                                    self.current_in_week = self.next_pending_in_week();
                                                    if self.current_in_week.is_none() {
                                                        self.state = AppState::Summary;
                                                    }
                                                } else {
                                                    self.questions[idx].fails += 1;
                                                    self.message = "❌ Incorrecto. Intenta de nuevo.".to_string();
                                                    self.input.clear();
                                                    // ¡NO actualices current_in_week aquí!
                                                }
                                                self.save_progress();
                                            }

                                        }


                                        if saltar.clicked() {
                                            self.questions[idx].skips += 1;
                                            self.questions[idx].attempts += 1;
                                            self.message = "⏩ Pregunta saltada. La verás más adelante.".to_string();
                                            self.input.clear();

                                            if !self.shown_this_round.contains(&idx) {
                                                self.shown_this_round.push(idx);
                                            }

                                            self.current_in_week = self.next_pending_in_week();
                                            if self.current_in_week.is_none() {
                                                self.state = AppState::Summary;
                                            }
                                            self.save_progress();
                                        }


                                        self.save_progress();
                                    });

                                    ui.horizontal(|ui| {
                                        let button_width = (panel_width - 8.0) / 2.0;
                                        let guardar = ui.add_sized([button_width, 36.0], egui::Button::new("Guardar y salir"));
                                        let terminar = ui.add_sized([button_width, 36.0], egui::Button::new("Terminar Quiz"));

                                        if terminar.clicked() {
                                            self.state = AppState::Summary;

                                        }

                                        if guardar.clicked() {
                                            self.save_progress();
                                            std::process::exit(0);
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
                egui::CentralPanel::default()
                    .show(ctx, |ui| {
                        let max_width = 600.0;
                        let panel_width = (ui.available_width() * 0.97).min(max_width);
                        let button_width = (panel_width) / 3.0;
                        let button_height = 36.0;
                        let total_height = 150.0 + 350.0 + 48.0;
                        let extra_space = (ui.available_height() - total_height).max(0.0) / 2.0;
                        ui.add_space(extra_space);

                        egui::Frame::default()
                            .fill(ui.visuals().window_fill())
                            .inner_margin(egui::Margin::symmetric(127, 20))
                            .show(ui, |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.heading("¡Fin del quiz!");
                                    ui.add_space(10.0);
                                    ui.label("Resumen de preguntas:");
                                    ui.add_space(5.0);

                                    let max_height = 350.0;
                                    egui::ScrollArea::vertical()
                                        .max_height(max_height)
                                        .max_width(max_width)
                                        .show(ui, |ui| {

                                            ui.horizontal(|ui| {
                                                ui.add_space(85.0);
                                                egui::Grid::new("quiz_results_grid")
                                                    .striped(true)
                                                    .spacing([8.0, 0.0])
                                                    .show(ui, |ui| {
                                                        // Cabeceras
                                                        ui.label("Pregunta");
                                                        ui.label("Intentos");
                                                        ui.label("Fallos");
                                                        ui.label("Saltos");
                                                        ui.label("Solución vista");
                                                        ui.label("Estado");
                                                        ui.end_row();

                                                        for (i, q) in self.questions.iter().enumerate() {
                                                            let status = if q.is_done && !q.saw_solution && q.fails == 0 {
                                                                "✅ Correcta"
                                                            } else if q.is_done && !q.saw_solution && q.fails > 0 {
                                                                "❌ Fallida"
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
                                            })




                                        });

                                    ui.add_space(20.0);


                                    ui.horizontal(|ui| {
                                        ui.add_space(75.0);
                                        let volver = ui.add_sized([button_width, button_height], egui::Button::new("Volver"));
                                        let terminar = ui.add_sized([button_width, button_height], egui::Button::new("Terminar"));

                                        if volver.clicked() {
                                            if let Some(lang) = self.selected_language {
                                                *self = QuizApp::load_progress(lang)
                                                    .unwrap_or_else(|| QuizApp::new_for_language(lang));
                                                self.state = AppState::Quiz;
                                            }
                                        }
                                        if terminar.clicked() {
                                            if let Some(lang) = self.selected_language {
                                                Self::delete_progress(lang);
                                            }
                                            *self = QuizApp::new();
                                        }
                                    })

                                });
                            });

                        ui.add_space(extra_space);
                    });
            }
        }
    }

}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "SummerQuiz - Telegram: @sugarRayL",
        options,
        Box::new(|cc| {
            // Detectar tema preferido del sistema
            let prefers_dark = cc.egui_ctx.style().visuals.dark_mode;
            if prefers_dark {
                cc.egui_ctx.set_visuals(Visuals::dark());
            } else {
                cc.egui_ctx.set_visuals(Visuals::light());
            }

            let quiz = QuizApp::new();
            Ok(Box::new(quiz))
        }),
    )
}



