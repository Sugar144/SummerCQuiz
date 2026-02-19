use super::*;

impl QuizApp {
    // Accesores seguros
    pub fn progress(&self) -> &QuizProgress {
        let lang = self.selected_language.expect("No language selected");
        self.progresses
            .get(&lang)
            .expect("No progress for selected language")
    }
    pub fn progress_mut(&mut self) -> &mut QuizProgress {
        let lang = self.selected_language.expect("No language selected");
        self.progresses
            .get_mut(&lang)
            .expect("No progress for selected language")
    }
    // Opcionales (útiles para guardas en UI)
    pub fn progress_opt(&self) -> Option<&QuizProgress> {
        self.selected_language.and_then(|l| self.progresses.get(&l))
    }
    pub fn progress_mut_opt(&mut self) -> Option<&mut QuizProgress> {
        self.selected_language
            .and_then(|l| self.progresses.get_mut(&l))
    }

    /// Sincroniza `is_done` en todas las preguntas anidadas a partir de `completed_ids`
    pub fn sync_is_done(&mut self) {
        if self.selected_language.is_none() {
            return;
        }
        let valid_ids: HashSet<String> = self.all_question_ids();
        {
            let prog = self.progress_mut();
            prog.completed_ids.retain(|id| valid_ids.contains(id));
        }
        let completed = self.progress().completed_ids.clone();
        for week in &mut self.quiz.weeks {
            for level in &mut week.levels {
                for q in &mut level.questions {
                    let already = q.is_done;
                    q.is_done =
                        q.id.as_ref()
                            .map(|id| completed.contains(id))
                            .unwrap_or(already);
                }
            }
        }
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
}
