use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use crate::code_utils::normalize_code;
use crate::model::{AppState, Language, Question};
use crate::data::read_questions_embedded;
use eframe::egui;

#[derive(Serialize, Deserialize, Clone)]
pub struct QuizProgress {
    pub completed_ids: HashSet<String>,
    pub current_week: Option<usize>,
    pub unlocked_weeks: Vec<usize>,
    pub max_unlocked_week: usize,
    pub current_in_week: Option<usize>,
    pub input: String,
    pub finished: bool,
    pub round: u32,
    pub shown_this_round: Vec<usize>,
    pub show_solution: bool,

}

impl Default for QuizProgress {
    fn default() -> Self {
        Self {
            completed_ids: HashSet::new(),
            current_week: None,
            unlocked_weeks: vec![1],
            max_unlocked_week: 1,
            current_in_week: None,
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
    pub questions: Vec<Question>,
    pub selected_language: Option<Language>,
    pub message: String,
    #[serde(skip)]
    pub state: AppState,
    #[serde(skip)]
    pub has_update: Option<String>,
    #[serde(skip)]
    pub confirm_reset: bool,
    #[serde(skip)]
    pub update_thread_launched:bool,
    #[serde(skip)]
    pub has_saved_progress: bool,
}

impl QuizApp {
    pub fn new() -> Self {
        let mut progresses = HashMap::new();
        progresses.insert(Language::C, QuizProgress::default());
        progresses.insert(Language::Pseudocode, QuizProgress::default());

        // Inicializa el struct principal
        let mut quiz_app = Self {
            progresses,
            selected_language: None,
            questions: vec![],
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

        let questions = read_questions_embedded()
            .into_iter()
            .filter(|q| q.language == language)
            .collect::<Vec<_>>();

        Self {
            progresses,
            selected_language: Some(language),
            questions,
            message: String::new(),
            state: AppState::Welcome,
            has_update: None,
            confirm_reset: false,
            update_thread_launched: false,
            has_saved_progress: false,
        }
    }


    pub fn select_week(&mut self, week: usize) {
        let language = self.selected_language.unwrap_or(Language::C);

        let current_in_week = self.questions
            .iter()
            .enumerate()
            .find(|(_, q)| q.week == week && q.language == language && !q.is_done)
            .map(|(idx, _)| idx);

        {
            let progress = self.progress_mut();

            if week > progress.max_unlocked_week {
                progress.max_unlocked_week = week;
            } else {
                if !progress.unlocked_weeks.contains(&week) {
                    progress.unlocked_weeks.push(week);
                    progress.unlocked_weeks.sort();
                }
            }

            progress.current_week = Some(week);
            progress.current_in_week = current_in_week;
            progress.round = 1;
            progress.shown_this_round.clear();
        }

        self.recalculate_unlocked_weeks();
    }




    // Al completar una semana, desbloquea la siguiente
    pub fn complete_week(&mut self, week: usize) {
        let language = self.selected_language.unwrap_or(Language::C);
        if self.is_week_completed(week) {
            let next_week = week + 1;
            if self.questions.iter().any(|q| q.week == next_week && q.language == language) {
                let progress = self.progress_mut();
                if next_week > progress.max_unlocked_week {
                    progress.max_unlocked_week = next_week;
                }
            }
            self.recalculate_unlocked_weeks(); // <-- ¡SIEMPRE LLAMAR AQUÍ!
        }
    }

    // Cambia el is_week_unlocked:
    pub fn is_week_unlocked(&self, week: usize) -> bool {
        self.progress().unlocked_weeks.contains(&week)
    }

    // Una semana está completa si todas sus preguntas están respondidas correctamente
    pub fn is_week_completed(&self, week: usize) -> bool {
        let language = self.selected_language.unwrap_or(Language::C);
        self.questions.iter()
            .filter(|q| q.week == week && q.language == language)
            .all(|q| q.id.as_ref().map(|id| self.progress().completed_ids.contains(id)).unwrap_or(false))
    }

    pub fn recalculate_unlocked_weeks(&mut self) {
        let progress = self.progress_mut();
        progress.unlocked_weeks.clear();
        for week in 1..=progress.max_unlocked_week {
            progress.unlocked_weeks.push(week);
        }
    }


    /// Busca la próxima pregunta pendiente, o None si no quedan
    pub fn next_pending_in_week(&mut self) -> Option<usize> {
        let week = self.progress().current_week;
        if let Some(week) = week {
            // Primero obtenemos los índices de las preguntas y su estado en un vector (solo lectura)
            let questions_data: Vec<(usize, bool)> = self.questions
                .iter()
                .enumerate()
                .map(|(idx, q)| (idx, q.week == week && !q.is_done))
                .collect();

            // Ahora el mutable borrow:
            let progress = self.progress_mut();

            for (idx, is_pending) in &questions_data {
                if *is_pending && !progress.shown_this_round.contains(idx) {
                    progress.shown_this_round.push(*idx);
                    return Some(*idx);
                }
            }

            // Si ya se han mostrado todas las pendientes, empieza nueva ronda
            if questions_data.iter().any(|&(_, is_pending)| is_pending) {
                progress.round += 1;
                progress.shown_this_round.clear();
                for (idx, is_pending) in &questions_data {
                    if *is_pending {
                        progress.shown_this_round.push(*idx);
                        return Some(*idx);
                    }
                }
            }
        }
        None
    }


    pub fn update_input_prefill(&mut self) {
        let idx = self.progress().current_in_week;
        let prefill = idx
            .and_then(|i| self.questions.get(i))
            .and_then(|q| q.input_prefill.clone());

        let progress = self.progress_mut();
        if let Some(text) = prefill {
            progress.input = text;
        } else {
            progress.input.clear();
        }
    }



    /// Borra progreso y reinicia el quiz para el lenguaje actual
    pub fn reset_progress(&mut self) {
        if let Some(language) = self.selected_language {
            *self = QuizApp::new_for_language(language);
            self.continuar_quiz();         // esto hace select_week(...) y setea Quiz
            self.confirm_reset = false;
            self.message.clear();
            self.has_saved_progress = false;
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

    pub fn cambiar_lenguaje(&mut self) {
        self.has_saved_progress = true;
        self.state = AppState::LanguageSelect;
    }

    /// Cambia el lenguaje y carga o inicializa el progreso, y va a la bienvenida
    pub fn seleccionar_lenguaje(&mut self, lang: Language) {
        self.selected_language = Some(lang);

        // 1. Cargar preguntas nuevas del lenguaje elegido (solo lectura)
        let questions = read_questions_embedded()
            .into_iter()
            .filter(|q| q.language == lang)
            .collect::<Vec<_>>();

        // 2. Calcula valid_ids y weeks ANTES de mutar nada
        let valid_ids: HashSet<_> = questions
            .iter()
            .filter_map(|q| q.id.as_ref())
            .cloned()
            .collect();

        let mut weeks: Vec<usize> = questions.iter().map(|q| q.week).collect();
        weeks.sort_unstable();
        weeks.dedup();

        // 3. Calcula qué preguntas deberían aparecer marcadas como completadas,
        //    leyendo primero el set de completed_ids para este lenguaje
        let completed_ids = self.progress().completed_ids.clone();

        let mut questions = questions; // ahora mutable
        for q in &mut questions {
            if let Some(id) = &q.id {
                q.is_done = completed_ids.contains(id);
            } else {
                q.is_done = false;
            }
        }

        // 4. Calcula el máximo de semana desbloqueada ANTES del borrow mutable
        let mut max_week = 1;
        for &w in &weeks {
            // is_week_completed accede a self, así que hazlo aquí fuera
            if self.is_week_completed(w) {
                max_week = max_week.max(w + 1);
            }
        }

        // 5. Calcula la posición a restaurar o la próxima pendiente (ANTES del borrow mutable)
        let (restored_week, restored_idx) = {
            let progress = self.progress();
            if let (Some(week), Some(idx)) = (progress.current_week, progress.current_in_week) {
                if week != 0 && idx < questions.len() && questions[idx].language == lang {
                    (Some(week), Some(idx))
                } else {
                    let next_week = questions.iter()
                        .filter(|q| q.language == lang && !q.is_done)
                        .map(|q| q.week)
                        .min();
                    (next_week, None)
                }
            } else {
                let next_week = questions.iter()
                    .filter(|q| q.language == lang && !q.is_done)
                    .map(|q| q.week)
                    .min();
                (next_week, None)
            }
        };

        // 6. Asigna las preguntas ya listas
        self.questions = questions;

        // 7. Ahora SÍ puedes pedir el borrow mutable para actualizar el progreso
        let progress = self.progress_mut();

        // Limpia completed_ids obsoletos (solo mantiene los ids que existen en las nuevas preguntas)
        progress.completed_ids.retain(|id| valid_ids.contains(id));

        // Actualiza máximo de semanas y desbloqueadas
        progress.max_unlocked_week = max_week;
        progress.unlocked_weeks.clear();
        for w in 1..=max_week {
            progress.unlocked_weeks.push(w);
        }

        // Restaura posición actual
        progress.current_week = restored_week;
        progress.current_in_week = restored_idx;

        // Limpia input y estado
        progress.input.clear();
        progress.round = 1;
        progress.shown_this_round.clear();
        progress.show_solution = false;
        progress.finished = false;

        // UI general
        self.state = AppState::Welcome;
        self.message.clear();
        self.has_saved_progress = true;
        self.sync_is_done();
    }

    pub fn continuar_quiz(&mut self) {
        // Usar el progreso actual por lenguaje
        let need_select = {
            let progress = self.progress();
            progress.current_week.is_none() || progress.current_in_week.is_none()
        };

        if need_select {
            if let Some(first_week) = self.questions.iter().filter(|q| !q.is_done).map(|q| q.week).min() {
                self.select_week(first_week);
                self.update_input_prefill();
            } else {
                let first_week = self.questions.iter().map(|q| q.week).min().unwrap_or(1);
                self.select_week(first_week);
                self.update_input_prefill();
            }
        }

        // Ahora muta el progreso y la UI
        {
            let progress = self.progress_mut();
            progress.finished = false;
            progress.input.clear();
        }
        self.state = AppState::Quiz;
        self.message.clear();
    }


    pub fn empezar_desde_cero(&mut self) {
        self.reset_progress();
        self.message.clear();
    }

    pub fn abrir_menu_semanal(&mut self) {
        self.sync_is_done();                // <--- Añade aquí
        self.recalculate_unlocked_weeks();
        self.state = AppState::WeekMenu;
    }

    pub fn salir_app(& mut self) {
        self.state = AppState::LanguageSelect;
    }

    pub fn acceder_a_semana(&mut self, week: usize) {
        self.select_week(week);
        self.state = AppState::Quiz;
        self.update_input_prefill();
        self.message.clear();
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
        let respuesta_correcta = user_code == answer_code;

        // Mutaciones sobre la pregunta, fuera del bloque mutable de progress
        if respuesta_correcta {
            self.questions[idx].is_done = true;
        } else {
            self.questions[idx].fails += 1;
        }

        // Lee y clona lo necesario antes del bloque mutable
        let question_id = self.questions[idx].id.clone();
        let need_update_shown = !self.progress().shown_this_round.contains(&idx);

        // Flags y datos para después del borrow
        let mut correcto = false;
        let mut summary_next = false;
        let mut need_call_sync = false;
        let mut curr_week = None;
        let mut set_pending_in_week = false;

        // --- Bloque de borrow mutable de progress ---
        {
            let progress = self.progress_mut();

            if need_update_shown {
                progress.shown_this_round.push(idx);
            }

            if respuesta_correcta {
                if let Some(id) = &question_id {
                    progress.completed_ids.insert(id.clone());
                }
                need_call_sync = true;
                progress.input.clear();
                set_pending_in_week = true; // Aplazamos la asignación
                curr_week = progress.current_week;
                correcto = true;
            } else {
                progress.input.clear();
            }
        }
        // --- Fin del bloque mutable de progress ---

        // Ahora puedes llamar a métodos de self y volver a mutar progress
        if set_pending_in_week {
            let next_idx = self.next_pending_in_week();
            let progress = self.progress_mut();
            progress.current_in_week = next_idx;
        }

        // Comprobar si hay que pasar al resumen final
        let week = curr_week.unwrap_or(1);
        if self.progress().current_in_week.is_none() && self.is_week_completed(week) {
            self.complete_week(week);
            summary_next = true;
        }

        if need_call_sync {
            self.sync_is_done();
        }
        self.update_input_prefill();

        if correcto {
            self.message = "✅ ¡Correcto!".to_string();
        } else {
            self.message = "❌ Incorrecto. Intenta de nuevo.".to_string();
        }
        if summary_next {
            self.state = AppState::Summary;
        }
    }

    pub fn saltar_pregunta(&mut self, idx: usize) {
        // Mutaciones sobre la pregunta
        self.questions[idx].skips += 1;
        self.questions[idx].attempts += 1;
        self.questions[idx].saw_solution = false;

        // Estado del progreso actual
        let week = self.progress().current_week.unwrap_or(1);
        let language = self.selected_language.unwrap_or(Language::C);

        // Indices de preguntas de la semana y lenguaje actual
        let indices: Vec<usize> = self.questions.iter().enumerate()
            .filter(|(_, q)| q.week == week && q.language == language)
            .map(|(i, _)| i)
            .collect();

        // Encuentra la siguiente pendiente
        let pos_in_week = indices.iter().position(|&i| i == idx);
        let next_idx = pos_in_week.and_then(|pos| {
            indices.iter().skip(pos + 1)
                .find(|&&i| !self.questions[i].is_done)
                .copied()
        });

        // Mutar solo el progreso
        {
            let progress = self.progress_mut();
            progress.current_in_week = next_idx;
            progress.input.clear();
        }

        self.message = "⏩ Pregunta saltada. La verás en la siguiente ronda.".to_string();
        self.update_input_prefill();

        // Verificar si toca ir al resumen
        let is_none = self.progress().current_in_week.is_none();
        if is_none && self.is_week_completed(week) {
            self.complete_week(week);
            self.state = AppState::Summary;
        }
    }

    pub fn avanzar_a_siguiente_pregunta(&mut self, idx: usize) {
        // Mutaciones sobre la pregunta
        self.questions[idx].saw_solution = true;

        // Estado del progreso actual
        let week = self.progress().current_week.unwrap_or(1);
        let language = self.selected_language.unwrap_or(Language::C);

        // Indices de preguntas de la semana y lenguaje actual
        let indices: Vec<usize> = self.questions.iter().enumerate()
            .filter(|(_, q)| q.week == week && q.language == language)
            .map(|(i, _)| i)
            .collect();

        // Encuentra la siguiente pendiente
        let pos_in_week = indices.iter().position(|&i| i == idx);
        let next_idx = pos_in_week.and_then(|pos| {
            indices.iter().skip(pos + 1)
                .find(|&&i| !self.questions[i].is_done)
                .copied()
        });

        // Mutar solo el progreso
        {
            let progress = self.progress_mut();
            progress.current_in_week = next_idx;
            progress.input.clear();
            progress.show_solution = false;
        }

        self.update_input_prefill();

        // Verificar si toca ir al resumen
        let is_none = self.progress().current_in_week.is_none();
        if is_none && self.is_week_completed(week) {
            self.complete_week(week);
            self.state = AppState::Summary;
        }
    }

    pub fn guardar_y_salir(&mut self) {
        self.has_saved_progress = true;
        self.state = AppState::Welcome;
        self.message.clear();
    }

    pub fn ver_progreso(&mut self) {
        self.state = AppState::Summary;
        self.message.clear();
    }

    /// Avanzar a la siguiente semana (prepara la UI y estado)
    pub fn avanzar_a_siguiente_semana(&mut self, current_week: usize) {
        let language = self.selected_language.unwrap_or(Language::C);

        // Busca todas las semanas para el lenguaje seleccionado, ordenadas y únicas
        let mut weeks: Vec<usize> = self.questions
            .iter()
            .filter(|q| q.language == language)
            .map(|q| q.week)
            .collect();
        weeks.sort_unstable();
        weeks.dedup();

        // Busca el índice de la semana actual y avanza a la siguiente
        if let Some(idx) = weeks.iter().position(|&w| w == current_week) {
            if let Some(&next_week) = weeks.get(idx + 1) {
                // Verifica si la siguiente semana está completada
                if self.is_week_completed(next_week) {
                    // Si está completada, lleva al menú de semanas
                    self.state = AppState::WeekMenu;
                    self.message = "La siguiente semana ya está completada. ¡Escoge otra desde el menú!".to_string();
                } else {
                    // Si NO está completada, entra normalmente
                    self.select_week(next_week);
                    self.recalculate_unlocked_weeks();
                    self.update_input_prefill();
                    self.state = AppState::Quiz;
                    self.message.clear();
                }
            } else {
                // No hay más semanas; podrías volver al menú, mostrar mensaje, etc.
                self.state = AppState::Welcome;
            }
        }
    }


    pub fn sync_is_done(&mut self) {
        let completed_ids = self.progress().completed_ids.clone(); // <-- clone para evitar double borrow

        for q in &mut self.questions {
            q.is_done = if let Some(id) = &q.id {
                completed_ids.contains(id)
            } else {
                false
            };
        }
    }

    pub fn nuevas_preguntas_en_semana(&self, semana: usize, language: Language) -> usize {
        let completed_ids = self.progress().completed_ids.clone(); // de nuevo, evitar borrow doble

        self.questions
            .iter()
            .filter(|q| q.week == semana && q.language == language)
            .filter(|q| {
                if let Some(id) = &q.id {
                    !completed_ids.contains(id)
                } else {
                    false
                }
            })
            .count()
    }


    pub fn hay_preguntas_nuevas(&self) -> bool {
        let language = self.selected_language.unwrap_or(Language::C);
        let completed_ids = self.progress().completed_ids.clone();

        let mut weeks: Vec<usize> = self.questions
            .iter()
            .filter(|q| q.language == language)
            .map(|q| q.week)
            .collect();
        weeks.sort_unstable();
        weeks.dedup();

        for &week in &weeks {
            if self.is_week_completed(week) {
                let hay_nueva = self.questions.iter()
                    .filter(|q| q.week == week && q.language == language)
                    .any(|q| {
                        if let Some(id) = &q.id {
                            !completed_ids.contains(id)
                        } else {
                            false
                        }
                    });
                if hay_nueva {
                    return true;
                }
            }
        }
        false
    }


    pub fn reiniciar_semana(&mut self, week: usize) {
        let language = self.selected_language.unwrap_or(Language::C);

        // 1. Obtén los ids a borrar (solo lectura de preguntas)
        let ids_a_borrar: Vec<String> = self.questions
            .iter()
            .filter(|q| q.week == week && q.language == language)
            .filter_map(|q| q.id.clone())
            .collect();

        // 2. Calcula la siguiente pregunta pendiente ANTES del bloque mutable
        let next_pending = self.questions
            .iter()
            .enumerate()
            .find(|(_, q)| q.week == week && q.language == language && !q.is_done)
            .map(|(idx, _)| idx);

        // 3. Elimina ids de completadas y resetea estado de progreso
        {
            let progress = self.progress_mut();

            for id in ids_a_borrar {
                progress.completed_ids.remove(&id);
            }
            progress.round = 1;
            progress.shown_this_round.clear();

            // Asigna aquí el resultado ya calculado
            progress.current_in_week = next_pending;
        }

        // 4. Marca preguntas como no hechas y resetea stats (mutar preguntas fuera del borrow mutable)
        for q in self.questions.iter_mut() {
            if q.week == week && q.language == language {
                q.is_done = false;
                q.attempts = 0;
                q.fails = 0;
                q.skips = 0;
                q.saw_solution = false;
            }
        }

        // 5. Actualiza el estado de preguntas a partir del set actual de completadas
        self.sync_is_done();
    }



    /// Obtiene el progreso mutable del lenguaje actual.
    pub fn progress_mut(&mut self) -> &mut QuizProgress {
        let lang = self.selected_language.expect("No language selected");
        self.progresses.get_mut(&lang).expect("No progress for selected language")
    }

    /// Obtiene el progreso de solo lectura del lenguaje actual.
    pub fn progress(&self) -> &QuizProgress {
        let lang = self.selected_language.expect("No language selected");
        self.progresses.get(&lang).expect("No progress for selected language")
    }

}
