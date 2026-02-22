use super::*;

impl QuizApp {
    pub fn complete_module(&mut self, module_idx: usize) {
        let language = self.selected_language.unwrap_or(Language::C);
        if self.is_module_completed(module_idx) {
            let next_module = module_idx + 1;
            // ¿Existe la siguiente semana y contiene al menos una pregunta de ese idioma?
            let has_questions = self
                .quiz
                .modules
                .get(next_module)
                .map(|w| {
                    w.levels
                        .iter()
                        .flat_map(|l| &l.questions)
                        .any(|q| q.language == language)
                })
                .unwrap_or(false);

            if has_questions {
                let progress = self.progress_mut();
                if next_module > progress.max_unlocked_module {
                    progress.max_unlocked_module = next_module;
                }
            }
            self.recalculate_unlocked_modules(); // ¡Siempre llamar aquí!
        }
    }

    /// Al completar un nivel, desbloquea el siguiente nivel en la misma semana
    pub fn complete_level(&mut self, module_idx: usize, level_idx: usize) {
        let language = self.selected_language.unwrap_or(Language::C);

        // Si está completado el nivel
        if self.is_level_completed(module_idx, level_idx) {
            let next_level = level_idx + 1;

            // ¿Existe el siguiente nivel y tiene preguntas de este idioma?
            let has_questions = self
                .quiz
                .modules
                .get(module_idx)
                .and_then(|w| w.levels.get(next_level))
                .map(|lvl| lvl.questions.iter().any(|q| q.language == language))
                .unwrap_or(false);

            if has_questions {
                // Desbloquea el siguiente nivel
                let progress = self.progress_mut();
                let levels = progress
                    .unlocked_levels
                    .entry(module_idx)
                    .or_insert_with(|| vec![0]);
                if !levels.contains(&next_level) {
                    levels.push(next_level);
                    levels.sort_unstable();
                }

                // Actualiza el máximo nivel desbloqueado de esta semana
                let max_level = progress.max_unlocked_level.entry(module_idx).or_insert(0);
                if next_level > *max_level {
                    *max_level = next_level;
                }
            } else {
                // Si no hay más niveles, ¡completó la semana!
                self.complete_module(module_idx);
            }
        }
    }

    // Cambia el is_module_unlocked:
    pub fn is_module_unlocked(&self, module_idx: usize) -> bool {
        self.progress().unlocked_modules.contains(&module_idx)
    }

    pub fn is_level_unlocked(&self, module: usize, level: usize) -> bool {
        self.progress()
            .unlocked_levels
            .get(&module)
            .map(|lvls| lvls.contains(&level))
            .unwrap_or(false)
    }

    // Una semana está completa si todas sus preguntas están respondidas correctamente
    pub fn is_module_completed(&self, module_idx: usize) -> bool {
        let language = self.selected_language.unwrap_or(Language::C);

        // Busca la semana
        if let Some(module) = self.quiz.modules.get(module_idx) {
            // Busca todas las preguntas del lenguaje en esa semana
            let all_completed = module
                .levels
                .iter()
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
    pub fn is_level_completed(&self, module_idx: usize, level_idx: usize) -> bool {
        let language = self.selected_language.unwrap_or(Language::C);

        self.quiz
            .modules
            .get(module_idx)
            .and_then(|module| module.levels.get(level_idx))
            .map(|level| {
                level
                    .questions
                    .iter()
                    .filter(|q| q.language == language)
                    .all(|q| {
                        q.id.as_ref()
                            .map(|id| self.progress().completed_ids.contains(id))
                            .unwrap_or(false)
                    })
            })
            .unwrap_or(false)
    }

    /// Desbloquea todas las semanas desde el índice 0 hasta max_unlocked_module (inclusive).
    pub fn recalculate_unlocked_modules(&mut self) {
        let prog = self.progress_mut();
        prog.unlocked_modules = (0..=prog.max_unlocked_module).collect();
    }

    /// Recalcula los niveles desbloqueados de cada semana según el máximo desbloqueado.
    /// Debe llamarse tras modificar max_unlocked_level o al restaurar el progreso.
    pub fn recalculate_unlocked_levels(&mut self, module_idx: usize) {
        let progress = self.progress_mut();
        // ¿Hasta qué nivel está desbloqueado en esta semana?
        let max_level = *progress.max_unlocked_level.get(&module_idx).unwrap_or(&0);
        let mut unlocked = vec![];
        for lvl in 0..=max_level {
            unlocked.push(lvl);
        }
        progress.unlocked_levels.insert(module_idx, unlocked);
    }

    pub fn nuevas_preguntas_en_semana(&self, module_idx: usize, language: Language) -> usize {
        let completed = &self.progress().completed_ids;
        self.quiz
            .modules
            .get(module_idx)
            .map(|module| {
                module
                    .levels
                    .iter()
                    .flat_map(|lvl| &lvl.questions)
                    .filter(|q| q.language == language)
                    .filter(|q| {
                        q.id.as_ref()
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
        for (wi, _) in self.quiz.modules.iter().enumerate() {
            if self.is_module_completed(wi) {
                // Si la semana wi está completada, miramos si hay pendientes
                if self.nuevas_preguntas_en_semana(wi, lang) > 0 {
                    return true;
                }
            }
        }
        false
    }

    pub fn has_next_module(&self) -> bool {
        let current_module = self.progress().current_module.unwrap_or(0);
        // Construir lista de semanas válidas para el lenguaje
        let valid_modules = self.valid_modules();
        let pos = valid_modules
            .iter()
            .position(|&i| i == current_module)
            .unwrap_or(0);
        pos + 1 < valid_modules.len()
    }

    /// Devuelve los índices de las semanas que contienen al menos una pregunta
    /// en el idioma seleccionado.
    pub(crate) fn valid_modules(&self) -> Vec<usize> {
        let lang = self.selected_language.unwrap_or(Language::C);
        self.quiz
            .modules
            .iter()
            .enumerate()
            .filter_map(|(i, wk)| {
                if wk
                    .levels
                    .iter()
                    .any(|lvl| lvl.questions.iter().any(|q| q.language == lang))
                {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Si no hay pregunta en curso, completa nivel y/o semana, y ajusta el estado.
    pub(crate) fn finalize_level_or_module(&mut self) {
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
            if self.is_module_completed(cw) {
                self.complete_module(cw);
                self.state = AppState::Summary;
            }
        }
    }
}
