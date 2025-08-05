use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use crate::code_utils::normalize_code;
use crate::model::{AppState, Language, Level, Question, Quiz, Week};
use crate::data::read_questions_embedded;
use eframe::egui;
use crate::update::descargar_binario_nuevo;

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
    pub update_thread_launched:bool,
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


    /// Selecciona una semana y posiciona en el primer nivel y pregunta pendiente (según desbloqueo)
    pub fn select_week(&mut self, week_idx: usize) {
        let language = self.selected_language.unwrap_or(Language::C);
        let quiz = &self.quiz;

        // Verifica que la semana exista
        let week = match quiz.weeks.get(week_idx) {
            Some(w) => w,
            None => return, // Semana inválida, no hacer nada
        };

        // Obtén los niveles desbloqueados para esta semana
        let unlocked_levels = self.progress()
            .unlocked_levels
            .get(&week_idx)
            .cloned()
            .unwrap_or_else(|| vec![0]);

        // Busca el primer nivel desbloqueado con preguntas pendientes
        let mut first_pending_level = 0;
        let mut found_pending = false;

        for &lvl_idx in &unlocked_levels {
            if let Some(level) = week.levels.get(lvl_idx) {
                for q in &level.questions {
                    if q.language == language {
                        if let Some(id) = &q.id {
                            if !self.progress().completed_ids.contains(id) {
                                first_pending_level = lvl_idx;
                                found_pending = true;
                                break;
                            }
                        }
                    }
                }
            }
            if found_pending {
                break;
            }
        }
        // Si no encontró pendiente, coge el primer nivel desbloqueado
        let select_level = if found_pending {
            first_pending_level
        } else {
            *unlocked_levels.get(0).unwrap_or(&0)
        };

        // Llama a select_level para posicionar al usuario en la primera pregunta pendiente del nivel elegido
        self.select_level(week_idx, select_level);
    }

    /// Selecciona un nivel dentro de una semana y posiciona en la primera pregunta pendiente
    pub fn select_level(&mut self, week_idx: usize, level_idx: usize) {
        let language = self.selected_language.unwrap_or(Language::C);
        let quiz = &self.quiz;

        // Verifica que la semana y el nivel existan
        let week = match quiz.weeks.get(week_idx) {
            Some(w) => w,
            None => return,
        };
        let level = match week.levels.get(level_idx) {
            Some(l) => l,
            None => return,
        };

        // Busca la primera pregunta pendiente en ese nivel
        let mut first_pending_question = None;
        for (q_idx, q) in level.questions.iter().enumerate() {
            if q.language == language {
                if let Some(id) = &q.id {
                    if !self.progress().completed_ids.contains(id) {
                        first_pending_question = Some(q_idx);
                        break;
                    }
                }
            }
        }
        // Si no encuentra pendiente, va a la primera pregunta
        let select_question = first_pending_question.unwrap_or(0);

        // Actualiza el progreso
        let progress = self.progress_mut();
        progress.current_week = Some(week_idx);
        progress.current_level = Some(level_idx);
        progress.current_in_level = Some(select_question);
        progress.round = 1;
        progress.shown_this_round.clear();
    }



    // Al completar una semana, desbloquea la siguiente
    pub fn complete_week(&mut self, week_idx: usize) {
        let language = self.selected_language.unwrap_or(Language::C);
        if self.is_week_completed(week_idx) {
            let next_week = week_idx + 1;
            // ¿Existe la siguiente semana y contiene al menos una pregunta de ese idioma?
            let has_questions = self.quiz.weeks.get(next_week)
                .map(|w| w.levels.iter().flat_map(|l| &l.questions)
                    .any(|q| q.language == language))
                .unwrap_or(false);

            if has_questions {
                let progress = self.progress_mut();
                if next_week > progress.max_unlocked_week {
                    progress.max_unlocked_week = next_week;
                }
            }
            self.recalculate_unlocked_weeks(); // ¡Siempre llamar aquí!
        }
    }

    /// Al completar un nivel, desbloquea el siguiente nivel en la misma semana
    pub fn complete_level(&mut self, week_idx: usize, level_idx: usize) {
        let language = self.selected_language.unwrap_or(Language::C);

        // Si está completado el nivel
        if self.is_level_completed(week_idx, level_idx) {
            let next_level = level_idx + 1;

            // ¿Existe el siguiente nivel y tiene preguntas de este idioma?
            let has_questions = self.quiz.weeks.get(week_idx)
                .and_then(|w| w.levels.get(next_level))
                .map(|lvl| lvl.questions.iter().any(|q| q.language == language))
                .unwrap_or(false);

            if has_questions {
                // Desbloquea el siguiente nivel
                let progress = self.progress_mut();
                let levels = progress.unlocked_levels.entry(week_idx).or_insert_with(|| vec![0]);
                if !levels.contains(&next_level) {
                    levels.push(next_level);
                    levels.sort_unstable();
                }

                // Actualiza el máximo nivel desbloqueado de esta semana
                let max_level = progress.max_unlocked_level.entry(week_idx).or_insert(0);
                if next_level > *max_level {
                    *max_level = next_level;
                }
            } else {
                // Si no hay más niveles, ¡completó la semana!
                self.complete_week(week_idx);
            }
        }
    }


    // Cambia el is_week_unlocked:
    pub fn is_week_unlocked(&self, week_idx: usize) -> bool {
        self.progress().unlocked_weeks.contains(&week_idx)
    }

    pub fn is_level_unlocked(&self, week: usize, level: usize) -> bool {
        self.progress().unlocked_levels
            .get(&week)
            .map(|lvls| lvls.contains(&level))
            .unwrap_or(false)
    }


    // Una semana está completa si todas sus preguntas están respondidas correctamente
    pub fn is_week_completed(&self, week_idx: usize) -> bool {
        let language = self.selected_language.unwrap_or(Language::C);

        // Busca la semana
        if let Some(week) = self.quiz.weeks.get(week_idx) {
            // Busca todas las preguntas del lenguaje en esa semana
            let all_completed = week.levels.iter()
                .flat_map(|level| &level.questions)
                .filter(|q| q.language == language)
                .all(|q| {
                    if let Some(id) = &q.id {
                        self.progress().completed_ids.contains(id)
                    } else {
                        false
                    }
                });
            return all_completed;
        }
        false
    }

    /// Comprueba si un nivel está completamente respondido
    pub fn is_level_completed(&self, week_idx: usize, level_idx: usize) -> bool {
        let language = self.selected_language.unwrap_or(Language::C);

        self.quiz.weeks.get(week_idx)
            .and_then(|week| week.levels.get(level_idx))
            .map(|level| {
                level.questions.iter()
                    .filter(|q| q.language == language)
                    .all(|q| q.id.as_ref().map(|id| self.progress().completed_ids.contains(id)).unwrap_or(false))
            })
            .unwrap_or(false)
    }


    /// Desbloquea todas las semanas desde el índice 0 hasta max_unlocked_week (inclusive).
    pub fn recalculate_unlocked_weeks(&mut self) {
        let prog = self.progress_mut();
        prog.unlocked_weeks = (0..=prog.max_unlocked_week).collect();
    }

    /// Recalcula los niveles desbloqueados de cada semana según el máximo desbloqueado.
    /// Debe llamarse tras modificar max_unlocked_level o al restaurar el progreso.
    pub fn recalculate_unlocked_levels(&mut self, week_idx: usize) {
        let progress = self.progress_mut();
        // ¿Hasta qué nivel está desbloqueado en esta semana?
        let max_level = *progress.max_unlocked_level.get(&week_idx).unwrap_or(&0);
        let mut unlocked = vec![];
        for lvl in 0..=max_level {
            unlocked.push(lvl);
        }
        progress.unlocked_levels.insert(week_idx, unlocked);
    }

    /// Busca la próxima pregunta pendiente, o None si no quedan
    pub fn next_pending_in_week(&mut self) -> Option<(usize, usize)> {
        let progress = self.progress();
        let week_idx = progress.current_week?;
        let language = self.selected_language.unwrap_or(Language::C);

        let week = self.quiz.weeks.get(week_idx)?;

        for (level_idx, level) in week.levels.iter().enumerate() {
            for (q_idx, q) in level.questions.iter().enumerate() {
                if q.language == language {
                    if let Some(id) = &q.id {
                        if !self.progress().completed_ids.contains(id) {
                            return Some((level_idx, q_idx));
                        }
                    }
                }
            }
        }
        None
    }


    /// Busca el índice de la próxima pregunta pendiente dentro del nivel actual.
    /// Devuelve Some(idx) o None si ya no quedan.
    pub fn next_pending_in_level(&mut self) -> Option<usize> {
        let progress = self.progress();
        let week_idx = progress.current_week?;
        let level_idx = progress.current_level?;
        let language = self.selected_language.unwrap_or(Language::C);

        // Busca la semana y el nivel actuales
        let week: Week = self.quiz.weeks.get(week_idx)?.clone();
        let level = week.levels.get(level_idx)?;

        // Haz una copia de shown_this_round para evitar doble borrow
        let shown = progress.shown_this_round.clone();

        // Busca una pregunta pendiente no mostrada aún en esta ronda
        for (q_idx, q) in level.questions.iter().enumerate() {
            if q.language == language {
                if let Some(id) = &q.id {
                    if !self.progress().completed_ids.contains(id)
                        && !shown.contains(&(level_idx, q_idx))
                    {
                        // Marca como mostrada en esta ronda
                        let progress = self.progress_mut();
                        progress.shown_this_round.push((level_idx, q_idx));
                        return Some(q_idx);
                    }
                }
            }
        }

        // Si todas ya fueron mostradas en esta ronda pero aún hay pendientes, arranca ronda nueva
        let hay_pendientes = level.questions.iter().enumerate().any(|(_q_idx, q)| {
            q.language == language
                && q.id.as_ref().map(|id| !self.progress().completed_ids.contains(id)).unwrap_or(false)
        });

        if hay_pendientes {
            let mut progress = self.progress_mut().clone();
            progress.round += 1;
            progress.shown_this_round.clear();
            // Busca de nuevo (ahora con shown_this_round vacío)
            for (q_idx, q) in level.questions.iter().enumerate() {
                if q.language == language {
                    if let Some(id) = &q.id {
                        if !self.progress().completed_ids.contains(id) {
                            progress.shown_this_round.push((level_idx, q_idx));
                            return Some(q_idx);
                        }
                    }
                }
            }
        }
        None
    }



    pub fn update_input_prefill(&mut self) {
        let progress = self.progress();
        let week_idx = progress.current_week;
        let level_idx = progress.current_level;
        let q_idx = progress.current_in_level;

        let prefill = week_idx
            .and_then(|w| self.quiz.weeks.get(w))
            .and_then(|week| level_idx.and_then(|l| week.levels.get(l)))
            .and_then(|level| q_idx.and_then(|q| level.questions.get(q)))
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
            // 1) reconstruye sólo el estado para este idioma
            *self = QuizApp::new_for_language(language);

            // 2) vuelve a “sembrar” el progreso vacío para el otro idioma,
            //    así tu HashMap siempre tiene ambas claves
            let other = match language {
                Language::C => Language::Pseudocode,
                Language::Pseudocode => Language::C,
            };
            self.progresses.insert(other, QuizProgress::default());

            // 3) elige la primera semana/pregunta y pasa a Quiz
            self.continuar_quiz();

            // 4) limpia las banderas de UI
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
    /// Cambia el lenguaje y carga o inicializa el progreso, y va a la bienvenida
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
                let qs: Vec<Question> = lvl.questions
                    .into_iter()
                    .filter(|q| q.language == lang)
                    .map(|mut q| {
                        q.is_done = q.id.as_ref().map(|id| prev_completed.contains(id)).unwrap_or(false);
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
        let valid_ids: HashSet<String> = self.quiz.weeks
            .iter()
            .flat_map(|w| &w.levels)
            .flat_map(|l| &l.questions)
            .filter_map(|q| q.id.clone())
            .collect();

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





    /// 1) Continuar (o iniciar) el quiz: selecciona la primera pregunta pendiente si hace falta.
    pub fn continuar_quiz(&mut self) {
        // Decidir si hace falta seleccionar semana/nivel/pregunta
        let need_select = {
            let prog = self.progress();
            prog.current_week.is_none()
                || prog.current_level.is_none()
                || prog.current_in_level.is_none()
        };

        if need_select {
            // Encuentra la primera pregunta pendiente recorriendo semanas→niveles→preguntas
            let lang = self.selected_language.unwrap_or(Language::C);
            if let Some((wi, li, qi)) = self.quiz
                .weeks
                .iter()
                .enumerate()
                .find_map(|(wi, wk)| {
                    wk.levels.iter().enumerate().find_map(|(li, lvl)| {
                        lvl.questions.iter().enumerate().find_map(|(qi, q)| {
                            if q.language == lang {
                                if let Some(id) = &q.id {
                                    if !self.progress().completed_ids.contains(id) {
                                        return Some((wi, li, qi));
                                    }
                                }
                            }
                            None
                        })
                    })
                })
            {
                // select_week usará wi para posicionarse en (li,qi)
                self.select_week(wi);
                self.update_input_prefill();
            } else {
                // Ninguna pendiente: arrancamos en la semana 0
                self.select_week(0);
                self.update_input_prefill();
            }
        }

        // Ahora solo mutamos lo estrictamente necesario en progress
        {
            let prog = self.progress_mut();
            prog.finished = false;
            prog.input.clear();
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

    /// 2) Procesar la respuesta a la pregunta actual
    pub fn procesar_respuesta(&mut self, respuesta: &str) {
        if respuesta.trim().is_empty() {
            self.message = "⚠ Debes escribir una respuesta antes de enviar.".into();
            return;
        }

        // 1) Extraer índices actuales
        let (cw, cl, ci) = {
            let prog = self.progress();
            match (prog.current_week, prog.current_level, prog.current_in_level) {
                (Some(w), Some(l), Some(i)) => (w, l, i),
                _ => {
                    self.message = "Error interno: no hay pregunta seleccionada.".into();
                    return;
                }
            }
        };

        // 2) Normalizar código para comparar
        let user_code = normalize_code(respuesta);
        let answer_code = {
            let q = &self.quiz.weeks[cw].levels[cl].questions[ci];
            normalize_code(&q.answer)
        };

        // 3) Mutar la pregunta: intentos y fails / is_done
        {
            let q = &mut self.quiz.weeks[cw].levels[cl].questions[ci];
            q.attempts += 1;
            if user_code == answer_code {
                q.is_done = true;
            } else {
                q.fails += 1;
            }
        }
        let correcta = user_code == answer_code;

        // 4) Preparar clonados antes de mutar progress
        let question_id = self.quiz.weeks[cw].levels[cl].questions[ci].id.clone();
        let need_update_shown = {
            let prog = self.progress();
            !prog.shown_this_round.contains(&(cl, ci))
        };

        // 5) Bloque mutable de progress: marcar completado, shown_this_round, etc.
        let mut mark_pending = false;
        let mut curr_week = None;
        {
            let prog = self.progress_mut();
            if need_update_shown {
                prog.shown_this_round.push((cl, ci));
            }
            if correcta {
                if let Some(id) = &question_id {
                    prog.completed_ids.insert(id.clone());
                }
                prog.input.clear();
                mark_pending = true;
                curr_week = prog.current_week;
            } else {
                prog.input.clear();
            }
        }

        // 6) Si era correcta, pasamos a la siguiente pregunta de este nivel
        if mark_pending {
            let next_q = self.next_pending_in_level();
            let prog = self.progress_mut();
            prog.current_in_level = next_q;
        }

        // 7) ¿Terminó nivel o semana?
        if self.progress().current_in_level.is_none() {
            let week_idx = curr_week.unwrap_or(cw);
            // Completar nivel
            if self.is_level_completed(week_idx, cl) {
                self.complete_level(week_idx, cl);
            }
            // Completar semana y preparar summary
            if self.is_week_completed(week_idx) {
                self.complete_week(week_idx);
                self.state = AppState::Summary;
            }
        }

        // 8) Sincronizar y prefill, luego mensaje
        self.sync_is_done();
        self.update_input_prefill();
        self.message = if correcta {
            "✅ ¡Correcto!".into()
        } else {
            "❌ Incorrecto. Intenta de nuevo.".into()
        };
    }

    pub fn saltar_pregunta(&mut self) {
        // 1) Extraer índices actuales (o salir si no hay pregunta)
        let (cw, cl, ci) = match (
            self.progress().current_week,
            self.progress().current_level,
            self.progress().current_in_level,
        ) {
            (Some(w), Some(l), Some(i)) => (w, l, i),
            _ => return,
        };

        // 2) Registrar estadísticas en la pregunta actual
        {
            let q = &mut self.quiz.weeks[cw].levels[cl].questions[ci];
            q.skips += 1;
            q.attempts += 1;
            q.saw_solution = false;
        }

        // 3) Marcarla como mostrada en esta ronda
        {
            let prog = self.progress_mut();
            if !prog.shown_this_round.contains(&(cl, ci)) {
                prog.shown_this_round.push((cl, ci));
            }
        }

        // 4) Determinar siguiente pregunta con lógica de rondas
        let next_q = self.next_pending_in_level();

        // 5) Actualizar el índice y limpiar el input
        {
            let prog = self.progress_mut();
            prog.current_in_level = next_q;
            prog.input.clear();
        }

        // 6) Si no quedan preguntas pendientes en el nivel tras la ronda:
        if self.progress().current_in_level.is_none() {
            // 6a) Completar nivel (y desbloquear siguiente nivel o semana)
            if self.is_level_completed(cw, cl) {
                self.complete_level(cw, cl);
            }
            // 6b) Completar semana si toca, y pasar al resumen
            if self.is_week_completed(cw) {
                self.complete_week(cw);
                self.state = AppState::Summary;
            }
        }

        // 7) Actualizar prefill y mensaje
        self.update_input_prefill();
        self.message = "⏩ Pregunta saltada. La verás en la siguiente ronda.".to_string();
    }

    /// Marca la solución vista y avanza a la siguiente pendiente dentro del nivel;
    /// si era la última, completa nivel/semana y puede ir al resumen.
    pub fn avanzar_a_siguiente_pregunta(&mut self) {
        // 1) Extraer índices actuales
        let (cw, cl, ci) = match (
            self.progress().current_week,
            self.progress().current_level,
            self.progress().current_in_level,
        ) {
            (Some(w), Some(l), Some(i)) => (w, l, i),
            _ => return,
        };

        // 2) Marcar que vio la solución
        {
            let q = &mut self.quiz.weeks[cw].levels[cl].questions[ci];
            q.saw_solution = true;
        }

        // 3) Avanzar al siguiente pendiente en el nivel
        let next_q = self.next_pending_in_level();
        {
            let prog = self.progress_mut();
            prog.current_in_level = next_q;
            prog.input.clear();
            prog.show_solution = false;
        }

        // 4) Si no quedan más preguntas, completar nivel/semana
        if self.progress().current_in_level.is_none() {
            // completar nivel
            if self.is_level_completed(cw, cl) {
                self.complete_level(cw, cl);
            }
            // completar semana y mostrar resumen
            if self.is_week_completed(cw) {
                self.complete_week(cw);
                self.state = AppState::Summary;
            }
        }

        // 5) Prefill de input
        self.update_input_prefill();
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
    /// Avanza a la siguiente semana que tenga preguntas en el lenguaje actual
    pub fn avanzar_a_siguiente_semana(&mut self) {
        let lang = self.selected_language.unwrap_or(Language::C);

        // 1) Construir la lista de índices de semanas válidas para este lenguaje
        let valid_week_idxs: Vec<usize> = self
            .quiz
            .weeks
            .iter()
            .enumerate()
            .filter_map(|(wi, wk)| {
                let has_lang = wk.levels.iter().any(|lvl| {
                    lvl.questions.iter().any(|q| q.language == lang)
                });
                if has_lang { Some(wi) } else { None }
            })
            .collect();

        // 2) Obtener el week_idx actual y encontrar su posición en valid_week_idxs
        let curr = match self.progress().current_week {
            Some(w) => w,
            None => {
                // Si no hay semana actual, arrancamos por la primera válida
                if let Some(&first) = valid_week_idxs.first() {
                    self.select_week(first);
                    self.update_input_prefill();
                }
                return;
            }
        };
        let pos = match valid_week_idxs.iter().position(|&wi| wi == curr) {
            Some(p) => p,
            None => {
                // Si la semana actual dejó de ser válida, arrancamos por la primera
                if let Some(&first) = valid_week_idxs.first() {
                    self.select_week(first);
                    self.update_input_prefill();
                }
                return;
            }
        };

        // 3) Intentar avanzar al siguiente de esa lista
        if let Some(&next_wi) = valid_week_idxs.get(pos + 1) {
            if self.is_week_completed(next_wi) {
                // Ya completada: volvemos al menú de semanas
                self.state = AppState::WeekMenu;
                self.message = "La siguiente semana ya está completada. ¡Escoge otra desde el menú!".to_string();
            } else {
                // No completada: entramos en ella
                self.select_week(next_wi);
                self.recalculate_unlocked_weeks();
                self.update_input_prefill();
                self.state = AppState::Quiz;
                self.message.clear();
            }
        } else {
            // No hay siguiente semana válida
            self.state = AppState::Welcome;
        }
    }


    /// Sincroniza `is_done` en todas las preguntas anidadas a partir de `completed_ids`
    pub fn sync_is_done(&mut self) {
        // 1) IDs válidos tras el filtrado anidado
        let valid_ids: HashSet<String> = self
            .quiz
            .weeks
            .iter()
            .flat_map(|w| &w.levels)
            .flat_map(|l| &l.questions)
            .filter_map(|q| q.id.clone())
            .collect();

        // 2) Purga completed_ids
        {
            let prog = self.progress_mut();
            prog.completed_ids.retain(|id| valid_ids.contains(id));
        }

        // 3) Clonamos para no volver a mutar progress
        let completed = self.progress().completed_ids.clone();

        // 4) Ajustamos `is_done` en cada pregunta
        for week in &mut self.quiz.weeks {
            for level in &mut week.levels {
                for q in &mut level.questions {
                    q.is_done = q
                        .id
                        .as_ref()
                        .map(|id| completed.contains(id))
                        .unwrap_or(false);
                }
            }
        }
    }

    /// Cuenta cuántas preguntas sin resolver hay en una semana dada (por índice)
    pub fn nuevas_preguntas_en_semana(&self, week_idx: usize, language: Language) -> usize {
        let completed = &self.progress().completed_ids;
        self.quiz.weeks
            .get(week_idx)
            .map(|week| {
                week.levels
                    .iter()
                    .flat_map(|lvl| &lvl.questions)
                    .filter(|q| q.language == language)
                    .filter(|q| {
                        q.id
                            .as_ref()
                            .map(|id| !completed.contains(id))
                            .unwrap_or(false)
                    })
                    .count()
            })
            .unwrap_or(0)
    }


    /// Indica si hay preguntas nuevas (hechas tras completar una semana) en el lenguaje actual
    pub fn hay_preguntas_nuevas(&self) -> bool {
        let lang = self.selected_language.unwrap_or(Language::C);
        for (wi, _) in self.quiz.weeks.iter().enumerate() {
            if self.is_week_completed(wi) {
                // Si la semana wi está completada, miramos si hay pendientes
                if self.nuevas_preguntas_en_semana(wi, lang) > 0 {
                    return true;
                }
            }
        }
        false
    }


    /// Reinicia tot el progreso de una semana (por índice), dejando la posición en la primera pendiente
    pub fn reiniciar_semana(&mut self, week_idx: usize) {
        let lang = self.selected_language.unwrap_or(Language::C);

        // 1) Recopilar IDs de esa semana + lenguaje
        let ids_to_remove: Vec<String> = self.quiz.weeks
            .get(week_idx)
            .into_iter()
            .flat_map(|w| &w.levels)
            .flat_map(|lvl| &lvl.questions)
            .filter(|q| q.language == lang)
            .filter_map(|q| q.id.clone())
            .collect();

        // 2) Buscar primera pregunta pendiente en esa semana
        let first_pending = self.quiz.weeks
            .get(week_idx)
            .and_then(|week| {
                week.levels.iter().enumerate().find_map(|(li, lvl)| {
                    lvl.questions.iter().enumerate().find_map(|(qi, q)| {
                        if q.language == lang && !q.is_done {
                            Some((li, qi))
                        } else {
                            None
                        }
                    })
                })
            });

        // 3) Borrar IDs y resetear rondas en progress
        {
            let prog = self.progress_mut();
            for id in ids_to_remove {
                prog.completed_ids.remove(&id);
            }
            prog.round = 1;
            prog.shown_this_round.clear();
            // Posición inicial en esta semana
            prog.current_week = Some(week_idx);
            prog.current_level = first_pending.map(|(li, _)| li);
            prog.current_in_level = first_pending.map(|(_, qi)| qi);
        }

        // 4) Resetear flags y estadísticas en las preguntas anidadas
        if let Some(week) = self.quiz.weeks.get_mut(week_idx) {
            for level in &mut week.levels {
                for q in &mut level.questions {
                    if q.language == lang {
                        q.is_done = false;
                        q.attempts = 0;
                        q.fails = 0;
                        q.skips = 0;
                        q.saw_solution = false;
                    }
                }
            }
        }

        // 5) Volver a sincronizar
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

    pub fn ensure_update_thread(&mut self) {
        if self.update_thread_launched {
            return;
        }
        self.update_thread_launched = true;

        // El nombre del updater según plataforma
        let updater = if cfg!(windows) {
            "summer_quiz_updater.exe".to_string()
        } else {
            "./summer_quiz_updater".to_string()
        };

        // Hilo que descarga y arranca el updater
        std::thread::spawn(move || {
            match descargar_binario_nuevo() {
                Ok(()) => {
                    // Pequeña pausa para que el mensaje se vea
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    // Lanza el updater y sale
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

    /// Devuelve referencia a una semana concreta
    pub fn week(&self, n: usize) -> Option<&Week> {
        self.quiz.weeks.iter().find(|w| w.number == n)
    }

    /// Devuelve referencia a un nivel concreto de una semana concreta
    pub fn level(&self, week: usize, level: usize) -> Option<&Level> {
        self.week(week)?.levels.iter().find(|l| l.number == level)
    }

    /// Devuelve las preguntas de un nivel concreto de una semana concreta
    pub fn questions_for(&self, week: usize, level: usize) -> Option<&Vec<Question>> {
        self.level(week, level).map(|l| &l.questions)
    }

    /// Devuelve *mut* si necesitas modificar
    pub fn questions_for_mut(&mut self, week: usize, level: usize) -> Option<&mut Vec<Question>> {
        self.quiz.weeks.iter_mut()
            .find(|w| w.number == week)?
            .levels.iter_mut()
            .find(|l| l.number == level)
            .map(|l| &mut l.questions)
    }

    // Si quieres aplanar todas las preguntas (muy útil para stats globales)
    pub fn all_questions(&self) -> Vec<&Question> {
        self.quiz.weeks
            .iter()
            .flat_map(|w| w.levels.iter())
            .flat_map(|l| l.questions.iter())
            .collect()
    }

}
