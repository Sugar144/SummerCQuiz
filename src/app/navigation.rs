use super::*;

impl QuizApp {
    pub fn cambiar_lenguaje(&mut self) {
        self.has_saved_progress = true;
        self.state = AppState::LanguageSelect;
    }

    pub fn select_week(&mut self, week_idx: usize) {
        let language = self.selected_language.unwrap_or(Language::C);
        let quiz = &self.quiz;

        // Verifica que la semana exista
        let week = match quiz.weeks.get(week_idx) {
            Some(w) => w,
            None => return, // Semana inválida, no hacer nada
        };

        // Obtén los niveles desbloqueados para esta semana
        let unlocked_levels = self
            .progress()
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

    /// Selecciona nivel y decide si mostrar teoría según el origen.
    pub fn select_level_with_origin(
        &mut self,
        week_idx: usize,
        level_idx: usize,
        entry: LevelEntry,
    ) {
        let language = self.selected_language.unwrap_or(Language::C);

        // 0) Validaciones
        if week_idx >= self.quiz.weeks.len() {
            return;
        }
        if level_idx >= self.quiz.weeks[week_idx].levels.len() {
            return;
        }

        let week = &self.quiz.weeks[week_idx];
        let level = &week.levels[level_idx];

        // 1) Buscar primera pregunta pendiente de ESTE lenguaje
        let mut first_pending_question: Option<usize> = None;
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

        // 2) Si vienes del menú y ya estabas en este (week, level), conserva la pregunta actual;
        //    si no, usa la primera pendiente (o 0 si no hay pendientes).
        let keep_existing_qi = entry == LevelEntry::Menu
            && self.progress().current_week == Some(week_idx)
            && self.progress().current_level == Some(level_idx);

        let select_question = if keep_existing_qi {
            self.progress()
                .current_in_level
                .unwrap_or_else(|| first_pending_question.unwrap_or(0))
        } else {
            first_pending_question.unwrap_or(0)
        };

        // 3) Actualizar progreso (sin resetear ronda si vienes del menú)
        {
            let prog = self.progress_mut();
            prog.current_week = Some(week_idx);
            prog.current_level = Some(level_idx);
            prog.current_in_level = Some(select_question);

            if entry != LevelEntry::Menu {
                prog.round = 1;
                prog.shown_this_round.clear();
            }
        }

        // 4) ¿Mostrar teoría?
        //    - Flow: mostrar si aún no se ha visto (week, level)
        //    - Restart: forzar mostrar
        //    - Menu: nunca mostrar (entra directo al quiz)
        let should_show_theory = match entry {
            LevelEntry::Flow => !self
                .progress()
                .seen_level_theory
                .contains(&(week_idx, level_idx)),
            LevelEntry::Restart => true,
            LevelEntry::Menu => false,
        };

        if should_show_theory {
            // Tu diseño actual: al cerrar teoría vuelves a LevelSummary (para que salga “Comenzar preguntas”)
            self.open_level_theory(AppState::LevelSummary);
        } else {
            // Entrar directo al quiz y refrescar el editor con el prefill correcto
            self.state = AppState::Quiz;
            self.update_input_prefill();
        }

        self.message.clear();
    }

    /// Wrapper para mantener compatibilidad: por defecto trata como Flow.
    pub fn select_level(&mut self, week_idx: usize, level_idx: usize) {
        self.select_level_with_origin(week_idx, level_idx, LevelEntry::Flow);
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

            if let Some(wi) = self.quiz.weeks.iter().enumerate().find_map(|(wi, wk)| {
                // ¿Algún nivel dentro de wk tiene una pregunta no completada?
                let has_pending = wk.levels.iter().flat_map(|lvl| &lvl.questions).any(|q| {
                    q.language == lang
                        && q.id
                            .as_ref()
                            .map(|id| !self.progress().completed_ids.contains(id))
                            .unwrap_or(false)
                });
                if has_pending { Some(wi) } else { None }
            }) {
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
        if self.state != AppState::LevelTheory {
            self.state = AppState::Quiz;
        }

        self.message.clear();
    }

    pub fn abrir_menu_semanal(&mut self) {
        self.sync_is_done();
        self.recalculate_unlocked_weeks();
        self.state = AppState::WeekMenu;
    }

    pub fn open_week_menu(&mut self) {
        // Asegura que los estados estén actualizados
        self.sync_is_done();
        self.recalculate_unlocked_weeks();
        self.state = AppState::WeekMenu;
        self.message.clear();
    }

    pub fn open_level_menu(&mut self) {
        // 1) Obtener la semana actual
        let week_idx = match self.progress().current_week {
            Some(w) => w,
            None => return,
        };

        // 2) Asegurar estructuras auxiliares
        {
            let prog = self.progress_mut();
            prog.max_unlocked_level.entry(week_idx).or_insert(0);
        }

        // 3) Recalcular niveles desbloqueados (usa borrow mutable interno)
        self.recalculate_unlocked_levels(week_idx);

        // 4) Preservar posición si es válida; si no, fijar primer nivel desbloqueado
        let (must_init_level, new_level_opt) = {
            let prog = self.progress();
            let levels_len = self.quiz.weeks[week_idx].levels.len();

            // ¿tenemos un current_level válido para esta semana y desbloqueado?
            let keep_current = prog.current_week == Some(week_idx)
                && prog
                    .current_level
                    .map(|li| {
                        li < levels_len
                            && prog
                                .unlocked_levels
                                .get(&week_idx)
                                .map(|v| v.contains(&li))
                                .unwrap_or(false)
                    })
                    .unwrap_or(false);

            if keep_current {
                (false, None) // no tocar ni level ni in_level
            } else {
                // elegir el primer nivel desbloqueado (o 0 si no hay info)
                let first_lvl = prog
                    .unlocked_levels
                    .get(&week_idx)
                    .and_then(|v| v.first().copied())
                    .unwrap_or(0);
                (true, Some(first_lvl))
            }
        };

        if must_init_level {
            let prev_li = self.progress().current_level;
            let new_li = new_level_opt.unwrap_or(0);
            let prog = self.progress_mut();
            prog.current_level = Some(new_li);

            // Si cambiamos de nivel, sí conviene limpiar la pregunta seleccionada;
            // si no cambiamos, NO la toquemos para no perder el puntero.
            if prev_li != Some(new_li) {
                prog.current_in_level = None;
            }
        }
        // 5) Cambiar estado y limpiar mensaje
        self.state = AppState::LevelMenu;
        self.message.clear();
    }

    pub fn open_level_theory(&mut self, return_to: AppState) {
        self.theory_return_state = return_to;
        self.state = AppState::LevelTheory;
        self.message.clear();
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

    pub fn salir_app(&mut self) {
        self.state = AppState::LanguageSelect;
    }

    /// Avanzar a la siguiente semana (prepara la UI y estado)
    pub fn avanzar_a_siguiente_semana(&mut self) {
        // 1) Construir la lista de índices de semanas válidas
        let valid_week_idxs = self.valid_weeks();

        // 2) Obtener la posición actual o inicializar en la primera válida
        let pos = match self.position_or_init_first(&valid_week_idxs) {
            Some(p) => p,
            None => return, // ya arrancamos en la primera, nada más que hacer
        };

        // 3) Intentar avanzar al siguiente índice
        if let Some(&next_wi) = valid_week_idxs.get(pos + 1) {
            if self.is_week_completed(next_wi) {
                // siguiente semana ya completada: volvemos al menú
                self.state = AppState::WeekMenu;
                self.message =
                    "La siguiente semana ya está completada. ¡Escoge otra desde el menú!"
                        .to_owned();
            } else {
                // entramos en la siguiente semana
                self.acceder_a_semana(next_wi);
            }
        } else {
            // no hay siguiente semana válida
            self.state = AppState::Welcome;
        }
    }

    /// Avanza al siguiente nivel que tenga preguntas pendientes en la semana actual.
    /// Si no hay más niveles pendientes, va al resumen de semana.
    pub fn avanzar_a_siguiente_nivel(&mut self) {
        let lang = self.selected_language.unwrap_or(Language::C);

        // 1) Obtener el índice de semana actual
        let wi = match self.progress().current_week {
            Some(w) => w,
            None => return, // Sin semana seleccionada
        };

        // 2) Lista de niveles válidos (que contienen preguntas de este idioma)
        let valid_level_idxs: Vec<usize> = self.quiz.weeks[wi]
            .levels
            .iter()
            .enumerate()
            .filter_map(|(li, lvl)| {
                if lvl.questions.iter().any(|q| q.language == lang) {
                    Some(li)
                } else {
                    None
                }
            })
            .collect();

        // 3) Obtener la posición actual o inicializar en la primera válida
        let pos = match self.progress().current_level {
            Some(cl) => {
                if let Some(p) = valid_level_idxs.iter().position(|&li| li == cl) {
                    p
                } else {
                    if let Some(&first) = valid_level_idxs.first() {
                        self.select_level(wi, first);
                        self.update_input_prefill();
                    }
                    return;
                }
            }
            None => {
                if let Some(&first) = valid_level_idxs.first() {
                    self.select_level(wi, first);
                    self.update_input_prefill();
                }
                return;
            }
        };

        if let Some(&next_li) = valid_level_idxs.get(pos + 1) {
            if self.is_level_completed(wi, next_li) {
                // siguiente nivel ya completado: volvemos al menú de semanas
                self.state = AppState::WeekMenu;
                self.message =
                    "El siguiente nivel ya está completado. ¡Escoge otro desde el menú!".to_owned();
            } else {
                self.select_level(wi, next_li);
                self.recalculate_unlocked_levels(wi);
                self.update_input_prefill();
                self.state = AppState::Quiz;

                self.message.clear();
            }
        } else {
            // no hay siguiente nivel válido
            self.state = AppState::Summary;
        }
    }

    // Helpers de navegación interna
    fn position_or_init_first(&mut self, valid_idxs: &[usize]) -> Option<usize> {
        // 1) Intentar leer el week actual
        if let Some(curr) = self.progress().current_week {
            if let Some(pos) = valid_idxs.iter().position(|&wi| wi == curr) {
                return Some(pos);
            }
        }
        // 2) Si no hay week o no está en valid_idxs, arrancamos por la primera válida
        if let Some(&first) = valid_idxs.first() {
            self.select_week(first);
            self.update_input_prefill();
        }
        None
    }

    pub(crate) fn current_position(&self) -> Option<(usize, usize, usize)> {
        let prog = self.progress();
        match (prog.current_week, prog.current_level, prog.current_in_level) {
            (Some(w), Some(l), Some(i)) => Some((w, l, i)),
            _ => None,
        }
    }
}
