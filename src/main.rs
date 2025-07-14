use eframe::egui;
use serde::{ Serialize, Deserialize };

#[derive(Serialize, Deserialize)]
struct Question {
    week: usize,
    prompt: String,  // Preguntas
    answer: String,  // Respuestas
    is_done: bool,   // true si respondida correctamente
    attempts: u32,   // intentos totales (aciertos+fallos+saltos)
    fails: u32,      // respuestas incorrectas
    skips: u32,      // veces saltadas
}

/* Progreso */
fn save_progress(questions: &Vec<Question>) {
    let json = serde_json::to_string(questions).unwrap();
    std::fs::write("quiz_progress.json", json).unwrap();
}

fn load_progress() -> Option<Vec<Question>> {
    if let Ok(json) = std::fs::read_to_string("quiz_progress.json") {
        if let Ok(questions) = serde_json::from_str(&json) {
            return Some(questions);
        }
    }
    None
}

fn delete_progress() {
    let _ = std::fs::remove_file("quiz_progress.json");
}

fn has_saved_progress() -> bool {
    std::fs::metadata("quiz_progress.json").is_ok()
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
    let csv_data = include_str!("data/quiz_questions.csv");
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(csv_data.as_bytes());
    let mut questions = Vec::new();
    for result in rdr.records() {
        let record = result.unwrap();
        let week = record.get(0).unwrap().parse::<usize>().unwrap();
        let prompt = record.get(1).unwrap().trim().to_string();
        let answer = record.get(2).unwrap().trim().to_string();
        questions.push(Question {
            week,
            prompt,
            answer,
            is_done: false,
            attempts: 0,
            fails: 0,
            skips: 0,
        });
    }
    questions
}

enum AppState {
    Welcome,
    WeekMenu,
    Quiz,
    Summary,
}

struct QuizApp {
    questions: Vec<Question>,
    current_week: Option<usize>,
    current_in_week: Option<usize>, // ahora es Option para detectar el fin
    input: String,
    message: String,
    finished: bool,
    round: u32,
    shown_this_round: Vec<usize>,
    state: AppState,
}

impl QuizApp {
    fn new() -> Self {
        let questions = load_progress().unwrap_or_else(read_questions_embedded);
        let first = questions.iter().position(|q| !q.is_done);
        Self {
            questions,
            current_week: None,
            current_in_week: first,
            input: String::new(),
            message: String::new(),
            finished: false,
            round: 1,
            shown_this_round: vec![],
            state: AppState::Welcome,
        }
    }

    // Devuelve solo las preguntas de la semana activa (¡muy útil!)
    fn questions_this_week(&self) -> Vec<(usize, &Question)> {
        if let Some(week) = self.current_week {
            self.questions
                .iter()
                .enumerate()
                .filter(|(_, q)| q.week == week)
                .collect()
        } else {
            vec![]
        }
    }

    fn select_week(&mut self, week: usize) {
        self.current_week = Some(week);
        // Busca el primer índice relativo de pregunta pendiente en esa semana
        let questions = self.questions_this_week();
        self.current_in_week = questions
            .iter()
            .position(|(_, q)| !q.is_done);
        self.round = 1;
        self.shown_this_round.clear();
    }

    // Una semana está desbloqueada si todas las anteriores están completas, o es la primera
    pub fn is_week_unlocked(&self, week: usize) -> bool {
        if week == 1 { return true; }
        (1..week).all(|w| self.is_week_completed(w))
    }

    // Una semana está completa si todas sus preguntas están respondidas correctamente
    pub fn is_week_completed(&self, week: usize) -> bool {
        self.questions.iter().filter(|q| q.week == week).all(|q| q.is_done)
    }


    /// Busca la próxima pregunta pendiente, o None si no quedan
    fn next_pending(&mut self) -> Option<usize> {
        // Preguntas aún no respondidas y aún no mostradas en la ronda actual
        for (idx, q) in self.questions.iter().enumerate() {
            if !q.is_done && !self.shown_this_round.contains(&idx) {
                self.shown_this_round.push(idx);
                return Some(idx);
            }
        }
        // Si ya se han mostrado todas las pendientes, empieza nueva ronda
        if self.questions.iter().any(|q| !q.is_done) {
            self.round += 1;
            self.shown_this_round.clear();
            // Empieza nueva ronda y muestra la primera pendiente de la nueva ronda
            for (idx, q) in self.questions.iter().enumerate() {
                if !q.is_done {
                    self.shown_this_round.push(idx);
                    return Some(idx);
                }
            }
        }
        None
    }
}

impl eframe::App for QuizApp {

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // BOTÓN SUPERIOR DE REINICIAR (solo visible durante el quiz y resumen)
        if matches!(self.state, AppState::Quiz | AppState::Summary) {
            egui::TopBottomPanel::top("menu_panel").show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if ui.button("🔄 Borrar progreso y reiniciar").clicked() {
                        delete_progress();
                        *self = QuizApp::new();
                    }
                });
            });
        }

        match self.state {
            // ----------- BIENVENIDA -----------
            AppState::Welcome => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let max_width = 600.0;
                    let panel_width = (ui.available_width() * 0.97).min(max_width);
                    let button_width = (panel_width - 8.0) / 3.0;
                    let button_height = 36.0;
                    let total_height = 220.0;
                    let extra_space = (ui.available_height() - total_height).max(0.0) / 2.0;
                    ui.add_space(extra_space);

                    egui::Frame::none()
                        .fill(ui.visuals().window_fill())
                        .inner_margin(egui::Margin::symmetric(120.0, 20.0))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.heading("👋 ¡Bienvenido al Quiz de C!");
                                ui.add_space(10.0);
                                ui.label("¿Qué deseas hacer?");

                                let hay_guardado = has_saved_progress();

                                ui.vertical_centered(|ui| {
                                    ui.add_space(16.0);

                                    if hay_guardado {
                                        let continuar = ui.add_sized([button_width, button_height], egui::Button::new("▶ Continuar donde lo dejé"));

                                        if continuar.clicked() {
                                            self.state = AppState::Quiz;
                                            self.finished = false;
                                            self.current_in_week = self.next_pending();
                                            self.input.clear();
                                            self.message.clear();
                                        }
                                    }


                                    let empezar = ui.add_sized([button_width, button_height], egui::Button::new("🔄 Empezar de 0"));
                                    let menu_semanal = ui.add_sized([button_width, button_height], egui::Button::new(" 📅 Seleccionar Semana"));
                                    let salir = ui.add_sized([button_width, button_height], egui::Button::new("❌ Salir"));

                                    if empezar.clicked() {
                                        delete_progress();
                                        *self = QuizApp::new();
                                        self.state = AppState::Quiz;
                                        self.finished = false;
                                        self.current_in_week = self.next_pending();
                                        self.input.clear();
                                        self.message.clear();
                                    }

                                    if menu_semanal.clicked() {
                                        self.state = AppState::WeekMenu;
                                    }

                                    if salir.clicked() {
                                        std::process::exit(0);                                    }
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
                    egui::Frame::none()
                        .fill(ui.visuals().window_fill())
                        .inner_margin(egui::Margin::symmetric(40.0, 20.0))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.heading("Selecciona una semana");
                                ui.add_space(20.0);

                                let total_weeks = self.questions.iter().map(|q| q.week).max().unwrap_or(1);

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

                    egui::Frame::none()
                        .fill(ui.visuals().window_fill())
                        .inner_margin(egui::Margin::symmetric(120.0, 20.0))
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
                                    egui::ScrollArea::vertical()
                                        .max_height(max_input_height)
                                        .auto_shrink([false; 2])
                                        .show(ui, |ui| {
                                            ui.add(
                                                egui::TextEdit::multiline(&mut self.input)
                                                    .desired_width(panel_width)
                                                    .desired_rows(16)
                                            );
                                        });

                                    ui.add_space(1.0);

                                    // Botones
                                    ui.horizontal(|ui| {
                                        ui.add_space((ui.available_width() - panel_width) / 2.0);
                                        let button_width = (panel_width - 8.0) / 2.0;
                                        let enviar = ui.add_sized([button_width, 36.0], egui::Button::new("Enviar"));
                                        let saltar = ui.add_sized([button_width, 36.0], egui::Button::new("Saltar pregunta"));

                                        if enviar.clicked() {
                                            let user_code = normalize_code(&self.input);
                                            let answer_code = normalize_code(&self.questions[idx].answer);
                                            self.questions[idx].attempts += 1;
                                            if user_code == answer_code {
                                                self.questions[idx].is_done = true;
                                                self.message = "✅ ¡Correcto!".to_string();
                                            } else {
                                                self.questions[idx].fails += 1;
                                                self.message = "❌ Incorrecto. Intenta de nuevo en otra ronda.".to_string();
                                            }
                                            self.input.clear();
                                            self.current_in_week = self.next_pending();
                                            if self.current_in_week.is_none() {
                                                // Quiz terminado → pasa a resumen
                                                self.state = AppState::Summary;
                                            }
                                            save_progress(&self.questions);
                                        }
                                        if saltar.clicked() {
                                            self.questions[idx].skips += 1;
                                            self.questions[idx].attempts += 1;
                                            self.message = "⏩ Pregunta saltada. La verás más adelante.".to_string();
                                            self.input.clear();
                                            self.current_in_week = self.next_pending();
                                            if self.current_in_week.is_none() {
                                                self.state = AppState::Summary;
                                            }
                                            save_progress(&self.questions);
                                        }


                                        save_progress(&self.questions);
                                    });

                                    ui.horizontal(|ui| {
                                        let button_width = (panel_width - 8.0) / 2.0;
                                        let guardar = ui.add_sized([button_width, 36.0], egui::Button::new("Guardar y salir"));
                                        let terminar = ui.add_sized([button_width, 36.0], egui::Button::new("Terminar Quiz"));

                                        if terminar.clicked() {
                                            self.state = AppState::Summary;

                                        }

                                        if guardar.clicked() {
                                            save_progress(&self.questions);
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
                        let max_width = 400.0;
                        let panel_width = (ui.available_width() * 0.97).min(max_width);
                        let button_width = (panel_width) / 2.0;
                        let button_height = 36.0;
                        let total_height = 150.0 + 350.0 + 48.0;
                        let extra_space = (ui.available_height() - total_height).max(0.0) / 2.0;
                        ui.add_space(extra_space);

                        egui::Frame::none()
                            .fill(ui.visuals().window_fill())
                            .inner_margin(egui::Margin::symmetric(180.0, 20.0))
                            .show(ui, |ui| {
                                ui.vertical_centered_justified(|ui| {
                                    ui.heading("¡Fin del quiz!");
                                    ui.add_space(10.0);
                                    ui.label("Resumen de preguntas:");
                                    ui.add_space(5.0);

                                    let max_height = 350.0;
                                    egui::ScrollArea::vertical()
                                        .max_height(max_height)
                                        .max_width(max_width)
                                        .show(ui, |ui| {

                                            for (i, q) in self.questions.iter().enumerate() {
                                                ui.label(format!(
                                                    "Pregunta {}: intentos {}, fallos {}, saltos {}, {}",
                                                    i + 1,
                                                    q.attempts,
                                                    q.fails,
                                                    q.skips,
                                                    if q.is_done { "✅ Correcta" } else { "❌ Sin responder" }
                                                ));
                                            }


                                        });

                                    ui.add_space(20.0);

                                    ui.horizontal_centered(|ui| {
                                        let retomar = ui.add_sized([button_width, button_height], egui::Button::new("Retomar"));
                                        let salir = ui.add_sized([button_width, button_height], egui::Button::new("Salir"));

                                        if retomar.clicked() {
                                            *self = QuizApp::new();
                                            self.state = AppState::Quiz;
                                        }
                                        if salir.clicked() {
                                            delete_progress();
                                            self.state = AppState::Welcome;
                                        }
                                    });
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
        "C Quiz Game - Telegram: @sugarRayL",
        options,
        Box::new(|_cc| Ok(Box::new(QuizApp::new())),
        ))
}


