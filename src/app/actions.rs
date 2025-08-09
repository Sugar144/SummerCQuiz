use super::*;
use crate::code_utils::normalize_code;

impl QuizApp {
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

                if self.is_week_completed(week_idx) {
                    // la semana ya acabó, pasamos al resumen semanal
                    self.state = AppState::Summary;
                } else {
                    // nivel completo pero quedan más niveles → resumen de nivel
                    self.state = AppState::LevelSummary;
                }
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
        let (cw, cl, ci) = match self.current_position() {
            Some(pos) => pos,
            None      => return,
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
        self.finalize_level_or_week();

        // 7) Actualizar prefill y mensaje
        self.update_input_prefill();
        self.message = "⏩ Pregunta saltada. La verás en la siguiente ronda.".to_string();
    }

    /// Marca la solución vista y avanza a la siguiente pendiente dentro del nivel;
    /// si era la última, completa nivel/semana y puede ir al resumen.
    pub fn avanzar_a_siguiente_pregunta(&mut self) {
        // 1) Extraer índices actuales
        let (cw, cl, ci) = match self.current_position() {
            Some(pos) => pos,
            None      => return,
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
        self.finalize_level_or_week();

        // 5) Prefill de input
        self.update_input_prefill();
    }

    // TEST helpers
    pub fn complete_all_week(&mut self) {
        let wi = match self.progress().current_week { Some(w) => w, None => return };
        let lang = self.selected_language.unwrap_or(Language::C);

        // 1) Marcar cada pregunta y acumular sus IDs
        let mut ids = Vec::new();
        for lvl in &mut self.quiz.weeks[wi].levels {
            for q in &mut lvl.questions {
                if q.language == lang {
                    if let Some(id) = q.mark_done_test() {
                        ids.push(id);
                    }
                }
            }
        }

        // 2) Añadir a completed_ids
        {
            let prog = self.progress_mut();
            for id in ids {
                prog.completed_ids.insert(id);
            }
        }

        // 4) Desbloquear lógica de semana
        self.complete_week(wi);

        // 4) **Sincroniza** para propagar completed_ids → q.is_done
        self.sync_is_done();

        // 5) Ir al resumen semanal
        self.state = AppState::Summary;
    }

    /// Marca *todas* las preguntas del nivel actual como completadas y va al resumen de nivel o al resumen semanal si era el último nivel.
    pub fn complete_all_level(&mut self) {
        let wi = match self.progress().current_week  { Some(w) => w, None => return };
        let li = match self.progress().current_level { Some(l) => l, None => return };
        let lang = self.selected_language.unwrap_or(Language::C);

        // 1) Marcar cada pregunta del nivel y acumular IDs
        let mut ids = Vec::new();
        for q in &mut self.quiz.weeks[wi].levels[li].questions {
            if q.language == lang {
                if let Some(id) = q.mark_done_test() {
                    ids.push(id);
                }
            }
        }

        // 2) Añadir a completed_ids
        {
            let prog = self.progress_mut();
            for id in ids {
                prog.completed_ids.insert(id);
            }
        }

        // 4) Disparar lógica de completar nivel (desbloquear siguiente nivel o semana)
        self.complete_level(wi, li);

        // 4) **Sincroniza** para propagar completed_ids → q.is_done
        self.sync_is_done();

        // 5) Si esa acción completó la semana entera, vamos al resumen semanal;
        //    en caso contrario, abrimos el resumen de nivel.
        self.state = AppState::LevelSummary;
    }

    // Motores internos
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
            {
                let progress = self.progress_mut();
                progress.round += 1;
                progress.shown_this_round.clear();
            }

            // Busca de nuevo (ahora con shown_this_round vacío)
            for (q_idx, q) in level.questions.iter().enumerate() {
                if q.language == language {
                    if let Some(id) = &q.id {
                        if !self.progress().completed_ids.contains(id) {
                            let progress = self.progress_mut();
                            progress.shown_this_round.push((level_idx, q_idx));
                            return Some(q_idx);
                        }
                    }
                }
            }
        }
        None
    }
}

