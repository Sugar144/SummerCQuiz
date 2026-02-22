use crate::data::read_questions_for_language;
use crate::model::{AppState, Language, Level, Module, Question, Quiz};
use eframe::egui;
use egui_commonmark::CommonMarkCache;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// Submódulos
pub mod actions;
pub mod completion;
pub mod navigation;
pub mod progress;
pub mod queries;
pub mod resets;
pub mod updates;
pub mod view_models;

// Re-export de view models
pub use crate::view_models::{LevelInfo, ModuleInfo, QuestionRow};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LevelEntry {
    Flow,    // flujo normal (continuar/avanzar)
    Menu,    // click en el menú de niveles
    Restart, // reinicio explícito del nivel
}

#[derive(Serialize, Deserialize, Clone)]
pub struct QuizProgress {
    pub completed_ids: HashSet<String>,
    pub current_module: Option<usize>, // Índice de la semana (en el vector de modules)
    pub current_level: Option<usize>,  // Índice del nivel dentro de la semana seleccionada
    pub current_in_level: Option<usize>, // Índice de la pregunta dentro del nivel actual
    pub unlocked_modules: Vec<usize>,
    pub unlocked_levels: HashMap<usize, Vec<usize>>, // semana -> [niveles desbloqueados]
    pub max_unlocked_module: usize,
    pub max_unlocked_level: HashMap<usize, usize>,
    pub input: String,
    pub finished: bool,
    pub round: usize,
    pub shown_this_round: Vec<(usize, usize)>, // Ahora es mejor guardar pares (nivel, pregunta)
    pub show_solution: bool,
    pub seen_level_theory: HashSet<(usize, usize)>,
}

impl Default for QuizProgress {
    fn default() -> Self {
        let mut unlocked_levels = HashMap::new();
        unlocked_levels.insert(0, vec![0]); // Solo nivel 1 desbloqueado para la semana 1
        let mut max_unlocked_level = HashMap::new();
        max_unlocked_level.insert(0, 0); // Solo nivel 1 desbloqueado para la semana 1

        Self {
            completed_ids: HashSet::new(),
            current_module: Some(0),
            current_level: Some(0),
            current_in_level: None,
            unlocked_modules: vec![0], // Solo semana 1 desbloqueada
            unlocked_levels,
            max_unlocked_module: 0,
            max_unlocked_level,
            input: String::new(),
            finished: false,
            round: 1,
            shown_this_round: vec![],
            show_solution: false,
            seen_level_theory: HashSet::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct QuizApp {
    pub progresses: HashMap<Language, QuizProgress>,
    pub quiz: Quiz,
    pub selected_language: Option<Language>,
    pub message: String,
    #[serde(skip)]
    pub state: AppState,
    #[serde(skip)]
    pub theory_return_state: AppState,
    #[serde(skip)]
    pub cm_cache: CommonMarkCache,
    #[serde(skip)]
    pub has_update: Option<String>,
    #[serde(skip)]
    pub confirm_reset: bool,
    #[serde(skip)]
    pub update_thread_launched: bool,
    #[serde(skip)]
    pub has_saved_progress: bool,
}

impl QuizApp {
    pub fn new() -> Self {
        let mut progresses = HashMap::new();
        progresses.insert(Language::C, QuizProgress::default());
        progresses.insert(Language::Pseudocode, QuizProgress::default());
        progresses.insert(Language::Kotlin, QuizProgress::default());
        progresses.insert(Language::Java, QuizProgress::default());
        progresses.insert(Language::Rust, QuizProgress::default());
        progresses.insert(Language::Python, QuizProgress::default());

        let quiz = read_questions_for_language(Language::C);

        // Inicializa el struct principal
        let mut quiz_app = Self {
            progresses,
            selected_language: None,
            quiz,
            message: String::new(),
            state: AppState::LanguageSelect,
            theory_return_state: AppState::LevelMenu,
            cm_cache: CommonMarkCache::default(),
            has_update: None,
            confirm_reset: false,
            update_thread_launched: false,
            has_saved_progress: false,
        };

        // --- Esto es igual que antes ---
        let signal_path = std::path::Path::new(".update_success");
        if signal_path.exists() {
            quiz_app.message = format!(
                "¡Actualización a versión {} completada!",
                env!("CARGO_PKG_VERSION")
            );
            let _ = std::fs::remove_file(signal_path);
        }

        quiz_app
    }

    pub fn new_for_language(language: Language) -> Self {
        let mut progresses = HashMap::new();
        progresses.insert(language, QuizProgress::default());

        let quiz = read_questions_for_language(language);

        Self {
            progresses,
            selected_language: Some(language),
            quiz,
            message: String::new(),
            state: AppState::Welcome,
            theory_return_state: AppState::LevelMenu,
            cm_cache: CommonMarkCache::default(),
            has_update: None,
            confirm_reset: false,
            update_thread_launched: false,
            has_saved_progress: false,
        }
    }

    /// Entrypoint para cambiar idioma y reconstruir el banco filtrado
    pub fn seleccionar_lenguaje(&mut self, lang: Language) {
        self.selected_language = Some(lang);
        self.progresses
            .entry(lang)
            .or_insert_with(QuizProgress::default);

        let prev_completed = self.progress().completed_ids.clone();
        let mut quiz = read_questions_for_language(lang);

        for module in &mut quiz.modules {
            for level in &mut module.levels {
                for (i, q) in level.questions.iter_mut().enumerate() {
                    q.number = i + 1;
                    q.is_done =
                        q.id.as_ref()
                            .map(|id| prev_completed.contains(id))
                            .unwrap_or(false);
                }
            }
        }

        self.quiz = quiz;
        self.state = AppState::Welcome;
        self.message.clear();
        self.has_saved_progress = true;
    }
}
