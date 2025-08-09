use super::*;

impl QuizApp {
    pub fn complete_week(&mut self, week_idx: usize) {
        let language = self.selected_language.unwrap_or(Language::C);
        if self.is_week_completed(week_idx) {
            let next_week = week_idx + 1;
            // ¿Existe la siguiente semana y contiene al menos una pregunta de ese idioma?
            let has_questions = self
                .quiz
                .weeks
                .get(next_week)
                .map(|w| {
                    w.levels
                        .iter()
                        .flat_map(|l| &l.questions)
                        .any(|q| q.language == language)
                 })
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

    pub fn has_next_week(&self) -> bool {
        let current_week = self.progress().current_week.unwrap_or(0);
        // Construir lista de semanas válidas para el lenguaje
        let valid_weeks = self.valid_weeks();
        let pos = valid_weeks.iter().position(|&i| i == current_week).unwrap_or(0);
        pos + 1 < valid_weeks.len()
    }

    /// Devuelve los índices de las semanas que contienen al menos una pregunta
    /// en el idioma seleccionado.
    pub(crate) fn valid_weeks(&self) -> Vec<usize> {
        let lang = self.selected_language.unwrap_or(Language::C);
        self.quiz
            .weeks
            .iter()
            .enumerate()
            .filter_map(|(i, wk)| {
                if wk.levels.iter().any(|lvl| {
                    lvl.questions.iter().any(|q| q.language == lang)
                }) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Si no hay pregunta en curso, completa nivel y/o semana, y ajusta el estado.
    pub(crate) fn finalize_level_or_week(&mut self) {
        let (cw, cl, _) = match self.current_position() {
            Some(pos) => pos,
            None => return,
        };
        if self.progress().current_in_level.is_none() {
            // Completar nivel
            if self.is_level_completed(cw, cl) {
                self.complete_level(cw, cl);
            }
            // Si ahora la semana está completa, ir al resumen semanal
            if self.is_week_completed(cw) {
                self.complete_week(cw);
                self.state = AppState::Summary;
            }
        }
    }
}

