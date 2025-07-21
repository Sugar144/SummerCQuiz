use serde::{Deserialize, Serialize};
use crate::code_utils::normalize_code;
use crate::model::{AppState, Language, Question};
use crate::data::read_questions_embedded;

#[derive(Serialize, Deserialize)]
pub struct QuizApp {
    pub questions: Vec<Question>,
    pub selected_language: Option<Language>,
    pub current_week: Option<usize>,
    pub unlocked_weeks: Vec<usize>,
    pub max_unlocked_week: usize,
    pub current_in_week: Option<usize>,
    pub input: String,
    pub message: String,
    pub finished: bool,
    pub round: u32,
    pub shown_this_round: Vec<usize>,
    pub show_solution: bool,
    #[serde(skip)]
    pub state: AppState,
    #[serde(skip)]
    pub has_update: Option<String>,
    #[serde(skip)]
    pub confirm_reset: bool,
    #[serde(skip)]
    pub update_thread_launched:bool
}

impl QuizApp {
    pub fn new() -> Self {
        let mut questions = read_questions_embedded();

        for q in &mut questions {
            q.is_done = false;
            q.attempts = 0;
            q.fails = 0;
            q.skips = 0;
            q.saw_solution = false;
        }

        // Primero creas el struct (puedes llamarlo quiz_app, o self si lo prefieres)
        let mut quiz_app = Self {
            questions,
            selected_language: None,
            current_week: None,
            unlocked_weeks: vec![1],
            max_unlocked_week: 1,
            current_in_week: None,
            input: String::new(),
            message: String::new(),
            finished: false,
            round: 1,
            shown_this_round: vec![],
            show_solution: false,
            state: AppState::LanguageSelect,
            has_update: None,
            confirm_reset: false,
            update_thread_launched: false,
        };

        // Luego chequeas si hay actualización y pones el mensaje
        let signal_path = std::path::Path::new(".update_success");
        if signal_path.exists() {
            // ¡La versión que importa es la que corre AHORA!
            quiz_app.message = format!(
                "¡Actualización a versión {} completada!",
                env!("CARGO_PKG_VERSION")
            );
            let _ = std::fs::remove_file(signal_path);
        }

        // Devuelves el struct ya inicializado
        quiz_app
    }


    pub fn new_for_language(language: Language) -> Self {
        let mut questions = read_questions_embedded()
            .into_iter()
            .filter(|q| q.language == language)
            .collect::<Vec<_>>();

        // Forzar estado limpio al crear nuevo quiz
        for q in &mut questions {
            q.is_done = false;
            q.attempts = 0;
            q.fails = 0;
            q.skips = 0;
            q.saw_solution = false;
        }

        let first_week = questions.iter()
            .map(|q| q.week)
            .min()
            .unwrap_or(1);

        let current_in_week = questions
            .iter()
            .position(|q| q.week == first_week && !q.is_done);

        Self {
            questions,
            selected_language: Some(language),
            current_week: Some(first_week),
            unlocked_weeks: vec![1],
            max_unlocked_week: 1,
            current_in_week,
            input: String::new(),
            message: String::new(),
            finished: false,
            round: 1,
            shown_this_round: vec![],
            show_solution: false,
            state: AppState::Welcome,
            has_update: None,
            confirm_reset: false,
            update_thread_launched: false,
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

    pub fn select_week(&mut self, week: usize) {

        // Si la semana seleccionada es mayor que el max actual, actualiza el máximo
        if week > self.max_unlocked_week {
            self.max_unlocked_week = week;
            self.recalculate_unlocked_weeks();
        } else {
            // Asegúrate de que la semana seleccionada está en el vector de semanas desbloqueadas (por si acaso)
            if !self.unlocked_weeks.contains(&week) {
                self.unlocked_weeks.push(week);
                self.unlocked_weeks.sort();
            }
        }

        self.current_week = Some(week);
        let language = self.selected_language.unwrap_or(Language::C);

        // ¿Todas las preguntas de la semana están is_done? Resetea solo las de esa semana
        let week_done = self.questions
            .iter()
            .filter(|q| q.week == week && q.language == language)
            .all(|q| q.is_done);

        if week_done {
            for q in self.questions.iter_mut().filter(|q| q.week == week && q.language == language) {
                q.is_done = false;
                q.fails = 0;
                q.attempts = 0;
                q.skips = 0;
                q.saw_solution = false;
            }
        }

        // Selecciona la primera pendiente
        self.current_in_week = self.questions
            .iter()
            .enumerate()
            .find(|(_, q)| q.week == week && q.language == language && !q.is_done)
            .map(|(idx, _)| idx);
        self.round = 1;
        self.shown_this_round.clear();
    }



    // Al completar una semana, desbloquea la siguiente
    pub fn complete_week(&mut self, week: usize) {
        let language = self.selected_language.unwrap_or(Language::C);
        if self.is_week_completed(week) {
            let next_week = week + 1;
            if self.questions.iter().any(|q| q.week == next_week && q.language == language) {
                if next_week > self.max_unlocked_week {
                    self.max_unlocked_week = next_week;
                }
            }
            self.recalculate_unlocked_weeks(); // <-- ¡SIEMPRE LLAMAR AQUÍ!
        }
    }



    // Cambia el is_week_unlocked:
    pub fn is_week_unlocked(&self, week: usize) -> bool {
        self.unlocked_weeks.contains(&week)
    }


    // Una semana está completa si todas sus preguntas están respondidas correctamente
    pub fn is_week_completed(&self, week: usize) -> bool {
        let language = self.selected_language.unwrap_or(Language::C);
        let questions_in_week: Vec<_> = self.questions
            .iter()
            .filter(|q| q.week == week && q.language == language)
            .collect();

        if questions_in_week.is_empty() {
            // Si no hay preguntas para esa semana, NO puede estar completada
            false
        } else {
            questions_in_week.iter().all(|q| q.is_done)
        }
    }


    pub fn recalculate_unlocked_weeks(&mut self) {
        self.unlocked_weeks.clear();
        for week in 1..=self.max_unlocked_week {
            self.unlocked_weeks.push(week);
        }
    }

    /// Busca la próxima pregunta pendiente, o None si no quedan
    pub fn next_pending_in_week(&mut self) -> Option<usize> {
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

    pub fn update_input_prefill(&mut self) {
        if let Some(idx) = self.current_in_week {
            if let Some(prefill) = &self.questions[idx].input_prefill {
                self.input = prefill.clone();
            } else {
                self.input.clear();
            }
        }
    }

    /// Borra progreso y reinicia el quiz para el lenguaje actual
    pub fn reset_progress(&mut self) {
        if let Some(language) = self.selected_language {
            Self::delete_progress(language);
            *self = QuizApp::new_for_language(language);
            self.state = AppState::Quiz;
            self.update_input_prefill();
            self.confirm_reset = false;
            self.message.clear();
        }
    }

    pub fn confirm_reset(&mut self, ctx: &egui::Context) {
        egui::Window::new("Confirmar reinicio")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("¿Seguro que quieres borrar todo tu progreso? ¡Esta acción no se puede deshacer!");
                ui.horizontal(|ui| {
                    if ui.button("Sí, borrar").clicked() {
                        self.reset_progress();
                    }
                    if ui.button("No").clicked() {
                        self.confirm_reset = false;
                    }
                });
            });
    }

    pub fn borrar_y_reiniciar(&mut self, ctx: &egui::Context) {
        self.confirm_reset = true;
        self.confirm_reset(ctx); // Si quieres, el diálogo también puede ser llamado aquí
    }

    pub fn cambiar_lenguaje(&mut self) {
        self.save_progress();
        self.state = AppState::LanguageSelect;
    }

    /// Cambia el lenguaje y carga o inicializa el progreso, y va a la bienvenida
    pub fn seleccionar_lenguaje(&mut self, lang: Language) {
        self.selected_language = Some(lang);
        if let Some(progress) = QuizApp::load_progress(lang) {
            *self = progress;
        } else {
            *self = QuizApp::new_for_language(lang);
        }
        self.state = AppState::Welcome;
        self.message.clear();
    }

    pub fn continuar_quiz(&mut self) {
        // Busca la primera semana pendiente, o la más baja si ya terminó tot
        if self.current_week.is_none() || self.current_in_week.is_none() {
            if let Some(first_week) = self.questions.iter().filter(|q| !q.is_done).map(|q| q.week).min() {
                self.select_week(first_week);
                self.update_input_prefill();
            } else {
                let first_week = self.questions.iter().map(|q| q.week).min().unwrap_or(1);
                self.select_week(first_week);
                self.update_input_prefill();
            }
        }
        self.state = AppState::Quiz;
        self.finished = false;
        self.input.clear();
        self.message.clear();
    }

    pub fn empezar_desde_cero(&mut self) {
        self.reset_progress();
        self.message.clear();
    }

    pub fn abrir_menu_semanal(&mut self) {
        self.state = AppState::WeekMenu;
    }

    pub fn salir_app(&self) {
        std::process::exit(0);
    }

    pub fn acceder_a_semana(&mut self, week: usize) {
        self.select_week(week);
        self.state = AppState::Quiz;
        self.update_input_prefill();
        self.message.clear();
        self.save_progress();
    }

    pub fn volver_al_menu_principal(&mut self) {
        self.state = AppState::Welcome;
        self.message.clear();
    }

    pub fn procesar_respuesta(&mut self, respuesta: &str, idx: usize) {
        if respuesta.trim().is_empty() {
            self.message = "⚠ Debes escribir una respuesta antes de enviar.".to_string();
            return;
        }
        let user_code = normalize_code(respuesta);
        let answer_code = normalize_code(&self.questions[idx].answer);
        self.questions[idx].attempts += 1;

        if !self.shown_this_round.contains(&idx) {
            self.shown_this_round.push(idx);
        }

        if user_code == answer_code {
            self.message.clear();
            self.questions[idx].is_done = true;
            self.message = "✅ ¡Correcto!".to_string();
            self.input.clear();
            self.current_in_week = self.next_pending_in_week();
            self.update_input_prefill();

            if self.current_in_week.is_none() && self.is_week_completed(self.current_week.unwrap_or(1)) {
                let week = self.current_week.unwrap_or(1);
                self.complete_week(week);
                self.state = AppState::Summary;
            }
        } else {
            self.questions[idx].fails += 1;
            self.message = "❌ Incorrecto. Intenta de nuevo.".to_string();
            self.input.clear();
        }
        self.save_progress();
    }

    pub fn saltar_pregunta(&mut self, idx: usize) {
        self.questions[idx].skips += 1;
        self.questions[idx].attempts += 1;
        self.message = "⏩ Pregunta saltada. La verás más adelante.".to_string();
        self.input.clear();

        if !self.shown_this_round.contains(&idx) {
            self.shown_this_round.push(idx);
        }

        self.current_in_week = self.next_pending_in_week();
        self.update_input_prefill();

        if self.current_in_week.is_none() && self.is_week_completed(self.current_week.unwrap_or(1)) {
            let week = self.current_week.unwrap_or(1);
            self.complete_week(week);
            self.state = AppState::Summary;
        }
        self.save_progress();
    }

    pub fn avanzar_a_siguiente_pregunta(&mut self, idx: usize) {
        self.questions[idx].saw_solution = true;
        self.show_solution = false; // Reset
        self.input.clear();
        self.current_in_week = self.next_pending_in_week();
        self.update_input_prefill();
        if self.current_in_week.is_none() && self.is_week_completed(self.current_week.unwrap_or(1)) {
            let week = self.current_week.unwrap_or(1);
            self.complete_week(week);
            self.state = AppState::Summary;
        }
        self.save_progress();
    }

    pub fn guardar_y_salir(&mut self) {
        self.save_progress();
        self.state = AppState::Welcome;
        self.message.clear();
    }

    pub fn ver_progreso(&mut self) {
        self.state = AppState::Summary;
        self.message.clear();
    }

    /// Volver atrás al quiz (cargar progreso desde disco si existe)
    pub fn volver_a_quiz(&mut self) {
        if let Some(lang) = self.selected_language {
            *self = QuizApp::load_progress(lang)
                .unwrap_or_else(|| QuizApp::new_for_language(lang));
            self.state = AppState::Quiz;
        }
    }

    /// Avanzar a la siguiente semana (prepara la UI y estado)
    pub fn avanzar_a_siguiente_semana(&mut self, next_week: usize) {
        self.select_week(next_week);
        self.recalculate_unlocked_weeks();
        self.update_input_prefill();
        self.save_progress();
        self.state = AppState::Quiz;
        self.message.clear();
    }

    /// Terminar y volver al menú principal
    pub fn terminar_resumen(&mut self) {
        self.state = AppState::Welcome;
    }


}

pub fn progress_filename(language: Language) -> &'static str {
    match language {
        Language::C => "quiz_progress_c.json",
        Language::Pseudocode => "quiz_progress_pseudocode.json",
    }
}