use super::*;

impl QuizApp {
    pub fn module_infos(&self) -> Vec<ModuleInfo> {
        let lang = self.selected_language.unwrap_or(Language::C);
        self.quiz
            .modules
            .iter()
            .enumerate()
            .map(|(wi, wk)| {
                let unlocked = self.is_module_unlocked(wi);
                let completed = self.is_module_completed(wi);
                let new_count = self.nuevas_preguntas_en_semana(wi, lang);
                ModuleInfo {
                    idx: wi,
                    number: wk.number,
                    unlocked,
                    completed,
                    new_count,
                }
            })
            .collect()
    }

    pub fn level_infos_in_current_module(&self) -> Option<Vec<LevelInfo>> {
        let wi = self.progress().current_module?;
        let lang = self.selected_language.unwrap_or(Language::C);
        let module = self.quiz.modules.get(wi)?;
        Some(
            module
                .levels
                .iter()
                .enumerate()
                .map(|(li, lvl)| {
                    let unlocked = self.is_level_unlocked(wi, li);
                    let completed = self.is_level_completed(wi, li);
                    let new_count = lvl
                        .questions
                        .iter()
                        .filter(|q| q.language == lang)
                        .filter(|q| {
                            q.id.as_ref()
                                .map(|id| !self.progress().completed_ids.contains(id))
                                .unwrap_or(false)
                        })
                        .count();
                    LevelInfo {
                        idx: li,
                        number: lvl.number,
                        unlocked,
                        completed,
                        new_count,
                    }
                })
                .collect(),
        )
    }

    pub fn summary_rows_for_module(&self) -> Vec<QuestionRow> {
        let mut rows = Vec::new();
        let wi = match self.progress().current_module {
            Some(w) => w,
            None => return rows,
        };
        let lang = self.selected_language.unwrap_or(Language::C);
        if let Some(module) = self.quiz.modules.get(wi) {
            for lvl in &module.levels {
                for (qi, q) in lvl
                    .questions
                    .iter()
                    .enumerate()
                    .filter(|(_, q)| q.language == lang)
                {
                    let done =
                        q.id.as_ref()
                            .map(|id| self.progress().completed_ids.contains(id))
                            .unwrap_or(false);
                    rows.push(QuestionRow {
                        level_number: lvl.number,
                        question_index_1based: qi + 1,
                        attempts: q.attempts,
                        fails: q.fails,
                        skips: q.skips,
                        saw_solution: q.saw_solution,
                        done,
                    });
                }
            }
        }
        rows
    }
}
