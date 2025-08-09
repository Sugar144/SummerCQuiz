use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use eframe::egui;

use crate::data::read_questions_embedded;
use crate::model::{AppState, Language, Level, Question, Quiz, Week};

// Submódulos
pub mod progress;
pub mod navigation;
pub mod completion;
pub mod actions;
pub mod resets;
pub mod updates;
pub mod queries;
pub mod view_models;

// Re-export de view models
pub use crate::view_models::{WeekInfo, LevelInfo, QuestionRow};

#[derive(Serialize, Deserialize, Clone)]
pub struct QuizProgress {
    pub completed_ids: HashSet<String>,
    pub current_week: Option<usize>,             // Índice de la semana (en el vector de weeks)
    pub current_level: Option<usize>,            // Índice del nivel dentro de la semana seleccionada
    pub current_in_level: Option<usize>,         // Índice de la pregunta dentro del nivel actual
    pub unlocked_weeks: Vec<usize>,
    pub unlocked_levels: HashMap<usize, Vec<usize>>, // semana -> [niveles desbloqueados]
    pub max_unlocked_week: usize,
    pub max_unlocked_level: HashMap<usize, usize>,
    pub input: String,
    pub finished: bool,
    pub round: u32,
    pub shown_this_round: Vec<(usize, usize)>,   // Ahora es mejor guardar pares (nivel, pregunta)
    pub show_solution: bool,
}

impl Default for QuizProgress {
    fn default() -> Self {
        let mut unlocked_levels = HashMap::new();
        unlocked_levels.insert(0, vec![0]); // Solo nivel 1 desbloqueado para la semana 1
        let mut max_unlocked_level = HashMap::new();
        max_unlocked_level.insert(0, 0); // Solo nivel 1 desbloqueado para la semana 1

        Self {
            completed_ids: HashSet::new(),
            current_week: Some(0),
            current_level: Some(0),
            current_in_level: None,
            unlocked_weeks: vec![0], // Solo semana 1 desbloqueada
            unlocked_levels,
            max_unlocked_week: 0,
            max_unlocked_level,
            input: String::new(),
            finished: false,
            round: 1,
            shown_this_round: vec![],
            show_solution: false,
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

        let quiz = read_questions_embedded();

        // Inicializa el struct principal
        let mut quiz_app = Self {
            progresses,
            selected_language: None,
            quiz,
            message: String::new(),
            state: AppState::LanguageSelect,
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

        let quiz = read_questions_embedded();

        Self {
            progresses,
            selected_language: Some(language),
            quiz,
            message: String::new(),
            state: AppState::Welcome,
            has_update: None,
            confirm_reset: false,
            update_thread_launched: false,
            has_saved_progress: false,
        }
    }

    /// Entrypoint para cambiar idioma y reconstruir el banco filtrado
    pub fn seleccionar_lenguaje(&mut self, lang: Language) {
        // 0) Asigna el nuevo lenguaje antes de usar self.progress()
        self.selected_language = Some(lang);

        // 1) Asegura que exista progreso para este lenguaje
        self.progresses.entry(lang).or_insert_with(QuizProgress::default);

        // 2) Clona los IDs completados para reusar en el filtrado
        let prev_completed = self.progress().completed_ids.clone();

        // 3) Reconstruye self.quiz filtrando solo preguntas de `lang`
        let raw = read_questions_embedded();
        let mut weeks_filtered = Vec::new();
        for w in raw.weeks {
            let mut lvls = Vec::new();
            for mut lvl in w.levels {
                // Filtramos y renumeramos:
                let qs: Vec<Question> = lvl
                    .questions
                    .into_iter()
                    .filter(|q| q.language == lang)
                    .enumerate()
                    .map(|(i, mut q)| {
                        q.number  = i + 1;  // numeramos de 1 a N dentro de este nivel
                        q.is_done = q.id.as_ref()
                            .map(|id| prev_completed.contains(id))
                            .unwrap_or(false);
                        q
                    })
                    .collect();
                if !qs.is_empty() {
                    lvl.questions = qs;
                    lvls.push(lvl);
                }
            }
            if !lvls.is_empty() {
                weeks_filtered.push( Week {
                    number: w.number,
                    explanation: w.explanation,
                    levels: lvls,
                });
            }
        }
        self.quiz = Quiz { weeks: weeks_filtered };

        // 4) IDs válidos tras filtrado
        let valid_ids: HashSet<String> = self.all_question_ids();

        // 5) Calcula max_week_number (basado en Week.number)
        let mut max_week_number = 0;
        for week in &self.quiz.weeks {
            if week.levels
                .iter()
                .flat_map(|l| &l.questions)
                .all(|q| q.is_done)
            {
                max_week_number = max_week_number.max(week.number);
            }
        }
        // Desbloquea la siguiente semana numérica
        max_week_number += 1;

        // 6) Construye unlocked_idxs = índices 0-based de weeks cuyo `.number` <= max_week_number
        let unlocked_idxs: Vec<usize> = self.quiz.weeks
            .iter()
            .enumerate()
            .filter_map(|(idx, week)| {
                if week.number <= max_week_number {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();

        // 7) Decide tu posición de reanudación (rest_w, rest_l, rest_q)
        let (rest_w, rest_l, rest_q) = {
            let prog = self.progress();
            let cw = prog.current_week;
            let cl = prog.current_level;
            let ci = prog.current_in_level;

            let find_first = || {
                for (wi, wk) in self.quiz.weeks.iter().enumerate() {
                    for (li, lvl) in wk.levels.iter().enumerate() {
                        for (qi, q) in lvl.questions.iter().enumerate() {
                            if !q.is_done {
                                return (Some(wi), Some(li), Some(qi));
                            }
                        }
                    }
                }
                (None, None, None)
            };

            if let (Some(wi), Some(li), Some(qi)) = (cw, cl, ci) {
                if wi < self.quiz.weeks.len() {
                    let wk = &self.quiz.weeks[wi];
                    if li < wk.levels.len() && qi < wk.levels[li].questions.len() {
                        (Some(wi), Some(li), Some(qi))
                    } else {
                        find_first()
                    }
                } else {
                    find_first()
                }
            } else {
                find_first()
            }
        };

        // 8) Aplica tot al progreso en un solo bloque mutable
        {
            let prog = self.progress_mut();

            // a) conserva solo IDs que siguen existiendo
            prog.completed_ids.retain(|id| valid_ids.contains(id));

            // b) desbloquea semanas por índice
            prog.unlocked_weeks    = unlocked_idxs.clone();
            prog.max_unlocked_week = *unlocked_idxs.iter().max().unwrap_or(&0);

            // c) posición de reanudación
            prog.current_week     = rest_w;
            prog.current_level    = rest_l;
            prog.current_in_level = rest_q;

            // d) reset de UI/input/rondas
            prog.input.clear();
            prog.round = 1;
            prog.shown_this_round.clear();
            prog.show_solution = false;
            prog.finished = false;
        }

        // 9) Vuelve al menú de bienvenida
        self.state = AppState::Welcome;
        self.message.clear();
        self.has_saved_progress = true;
    }
}

