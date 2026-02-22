use super::*;

impl QuizApp {
    pub fn reset_progress(&mut self) {
        if let Some(language) = self.selected_language {
            // 1) reconstruye sólo el estado para este idioma
            *self = QuizApp::new_for_language(language);

            // 2) vuelve a “sembrar” el progreso vacío para el otro idioma,
            //    así tu HashMap siempre tiene ambas claves
            let other = match language {
                Language::C => Language::Pseudocode,
                Language::Pseudocode => Language::C,
                _ => Language::C,
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

    pub fn empezar_desde_cero(&mut self) {
        self.reset_progress();
        self.message.clear();
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

    /// Reinicia tot el progreso de una semana (por índice), dejando la posición en la primera pendiente
    pub fn reiniciar_semana(&mut self, week_idx: usize) {
        let lang = self.selected_language.unwrap_or(Language::C);

        // 1) Recopilar IDs de esa semana + lenguaje
        let ids_to_remove: Vec<String> = self
            .quiz
            .weeks
            .get(week_idx)
            .into_iter()
            .flat_map(|w| &w.levels)
            .flat_map(|lvl| &lvl.questions)
            .filter(|q| q.language == lang)
            .filter_map(|q| q.id.clone())
            .collect();

        // 2) Buscar primera pregunta pendiente en esa semana
        let first_pending = self.quiz.weeks.get(week_idx).and_then(|week| {
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
                    q.reset_stats();
                }
            }
        }

        // 5) Volver a sincronizar
        self.sync_is_done();
    }

    /// Reinicia tot el progreso de un nivel (por índice) dentro de una semana,
    /// dejando la posición en la primera pregunta pendiente de ese nivel.
    pub fn reiniciar_nivel(&mut self, week_idx: usize, level_idx: usize) {
        let lang = self.selected_language.unwrap_or(Language::C);

        // 1) Recopilar IDs de las preguntas de ese nivel + lenguaje
        let ids_to_remove: Vec<String> = self
            .quiz
            .weeks
            .get(week_idx)
            .and_then(|w| w.levels.get(level_idx))
            .into_iter()
            .flat_map(|lvl| &lvl.questions)
            .filter(|q| q.language == lang)
            .filter_map(|q| q.id.clone())
            .collect();

        // 2) Buscar la primera pregunta pendiente en ese nivel
        let first_pending = self
            .quiz
            .weeks
            .get(week_idx)
            .and_then(|w| w.levels.get(level_idx))
            .and_then(|lvl| {
                lvl.questions.iter().enumerate().find_map(|(qi, q)| {
                    if q.language == lang && !q.is_done {
                        Some(qi)
                    } else {
                        None
                    }
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
            // Posición inicial en este nivel
            prog.current_week = Some(week_idx);
            prog.current_level = Some(level_idx);
            prog.current_in_level = first_pending;
        }

        // 4) Resetear flags y estadísticas en las preguntas del nivel
        if let Some(lvl) = self
            .quiz
            .weeks
            .get_mut(week_idx)
            .and_then(|w| w.levels.get_mut(level_idx))
        {
            for q in &mut lvl.questions {
                q.reset_stats();
            }
        }


        // 5) Volver a sincronizar el estado global de is_done
        self.sync_is_done();
    }

    pub fn volver_niveles(&mut self) {
        self.has_saved_progress = true;
        self.state = AppState::LevelMenu;
        self.message.clear();
    }

    pub fn ver_progreso(&mut self) {
        self.state = AppState::LevelSummary;
        self.message.clear();
    }
}
