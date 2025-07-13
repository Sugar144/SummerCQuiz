use eframe::egui;
use serde::{ Serialize, Deserialize };

#[derive(Serialize, Deserialize)]
struct Question {
    prompt: String,  // Preguntas
    answer: String,  // Respuestas
    is_done: bool,   // true si respondida correctamente
    attempts: u32,   // intentos totales (aciertos+fallos+saltos)
    fails: u32,      // respuestas incorrectas
    skips: u32,      // veces saltada
}

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

fn normalize_code(input: &str) -> String {
    input
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.starts_with("//") && !line.is_empty())
        .collect::<Vec<_>>()
        .join("")
        .replace(char::is_whitespace, "")
        .to_lowercase()
}

fn read_questions_embedded() -> Vec<Question> {
    let csv_data = include_str!("data/quiz_questions.csv");
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(csv_data.as_bytes());
    let mut questions = Vec::new();
    for result in rdr.records() {
        let record = result.unwrap();
        let prompt = record.get(0).unwrap().to_string();
        let answer = record.get(1).unwrap().to_string();
        questions.push(Question {
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


struct QuizApp {
    questions: Vec<Question>,
    current: Option<usize>, // ahora es Option para detectar el fin
    input: String,
    message: String,
    finished: bool,
    round: u32,
    shown_this_round: Vec<usize>,
}

impl QuizApp {
    fn new() -> Self {
        let questions = load_progress().unwrap_or_else(read_questions_embedded);
        let first = questions.iter().position(|q| !q.is_done);
        Self {
            questions,
            current: first,
            input: String::new(),
            message: String::new(),
            finished: false,
            round: 1,
            shown_this_round: vec![],
        }
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
                        if self.finished {
                            ui.heading("¡Fin del quizz!");
                            ui.add_space(10.0);
                            ui.label("Resumen de preguntas:");
                            for (i, q) in self.questions.iter().enumerate() {
                                ui.label(format!(
                                    "Pregunta {}: intentos {}, fallos {}, saltos {}, {}",
                                    i+1, q.attempts, q.fails, q.skips,
                                    if q.is_done {"✅ Correcta"} else {"❌ Sin responder"}
                                ));
                            }
                        } else if let Some(idx) = self.current {
                            ui.heading(format!("🌀 Ronda {}", self.round));
                            // Prompt con scroll fijo como ya tienes (puedes copiar tu bloque)
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

                            // Botones y lógica
                            ui.horizontal(|ui| {
                                ui.add_space((ui.available_width() - panel_width) / 2.0);
                                let button_width = (panel_width - 8.0) / 3.0;
                                let enviar = ui.add_sized([button_width, 36.0], egui::Button::new("Enviar"));
                                let saltar = ui.add_sized([button_width, 36.0], egui::Button::new("Saltar"));
                                let terminar = ui.add_sized([button_width, 36.0], egui::Button::new("Terminar Quiz"));

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
                                    self.current = self.next_pending();
                                    // Si ya no quedan, termina
                                    if self.current.is_none() {
                                        self.finished = true;
                                    }

                                    save_progress(&self.questions); // <--- GUARDADO AUTOMÁTICO AQUÍ
                                }
                                if saltar.clicked() {
                                    self.questions[idx].skips += 1;
                                    self.questions[idx].attempts += 1;
                                    self.message = "⏩ Pregunta saltada. La verás más adelante.".to_string();
                                    self.input.clear();
                                    self.current = self.next_pending();
                                    if self.current.is_none() {
                                        self.finished = true;
                                    }
                                    save_progress(&self.questions); // <--- GUARDADO AUTOMÁTICO AQUÍ
                                }
                                if terminar.clicked() {
                                    self.finished = true;
                                }
                                save_progress(&self.questions); // <--- GUARDADO AUTOMÁTICO AQUÍ
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
}




fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "C Quiz Game - Brian Ferreira",
        options,
        Box::new(|_cc| Ok(Box::new(QuizApp::new())),
    ))
}


