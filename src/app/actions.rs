use super::*;
use crate::code_utils::normalize_code;
use crate::judge::judge_c::{
    JudgeResult, format_judge_message, grade_c_question, should_use_judge,
};
use crate::judge::judge_java::grade_java_question;
use crate::judge::judge_kt::grade_kotlin_question;
use crate::judge::judge_pseudo::{CJudge, PseudoConfig, run_pseudo_tests};
use crate::judge::judge_python::grade_python_question;
use crate::judge::judge_remote::grade_remote_question;
use crate::judge::judge_rust::grade_rust_question;
use crate::model::GradingMode;

impl QuizApp {
    pub fn procesar_respuesta(&mut self, respuesta: &str) {
        if respuesta.trim().is_empty() {
            self.message = "⚠ Debes escribir una respuesta antes de enviar.".into();
            return;
        }

        if self.remote_judge_pending.is_some() {
            self.message =
                "⏳ Ya hay una evaluación remota en progreso. Espera el resultado.".into();
            return;
        }

        let (cw, cl, ci) = {
            let prog = self.progress();
            match (
                prog.current_module,
                prog.current_level,
                prog.current_in_level,
            ) {
                (Some(w), Some(l), Some(i)) => (w, l, i),
                _ => {
                    self.message = "Error interno: no hay pregunta seleccionada.".into();
                    return;
                }
            }
        };

        let q = &self.quiz.modules[cw].levels[cl].questions[ci];

        // In WASM, any question requiring a compiler must go to the remote judge
        #[cfg(target_arch = "wasm32")]
        if q.uses_judge_remote() || q.needs_compiler_judge() {
            self.start_remote_judge(cw, cl, ci, respuesta.to_string());
            return;
        }

        let grading_result = self.grade_locally(q, respuesta);
        self.apply_grading_result(cw, cl, ci, grading_result);
    }

    fn grade_locally(&self, q: &Question, respuesta: &str) -> JudgeResult {
        if q.uses_judge_pseudo() {
            return run_pseudo_tests(respuesta, &q.tests, &PseudoConfig::default(), &CJudge);
        }

        if matches!(q.mode, Some(GradingMode::JudgeKotlin)) {
            return grade_kotlin_question(q, respuesta);
        }
        if matches!(q.mode, Some(GradingMode::JudgeJava)) {
            return grade_java_question(q, respuesta);
        }
        if matches!(q.mode, Some(GradingMode::JudgeRust)) {
            return grade_rust_question(q, respuesta);
        }
        if matches!(q.mode, Some(GradingMode::JudgePython)) {
            return grade_python_question(q, respuesta);
        }

        #[cfg(not(target_arch = "wasm32"))]
        if q.uses_judge_remote() {
            return grade_remote_question(q, respuesta);
        }

        if should_use_judge(q) {
            return grade_c_question(q, respuesta);
        }

        let user_code = normalize_code(respuesta);
        let answer_code = normalize_code(&q.answer);
        if user_code == answer_code {
            JudgeResult::Accepted
        } else {
            JudgeResult::WrongAnswer {
                test_index: 0,
                input: String::new(),
                expected: String::new(),
                received: String::new(),
                diff: String::new(),
            }
        }
    }

    fn apply_grading_result(
        &mut self,
        cw: usize,
        cl: usize,
        ci: usize,
        grading_result: JudgeResult,
    ) {
        let correcta = matches!(grading_result, JudgeResult::Accepted);

        {
            let q = &mut self.quiz.modules[cw].levels[cl].questions[ci];
            q.attempts += 1;
            if correcta {
                q.is_done = true;
            } else {
                q.fails += 1;
            }
        }

        let question_id = self.quiz.modules[cw].levels[cl].questions[ci].id.clone();
        let need_update_shown = {
            let prog = self.progress();
            !prog.shown_this_round.contains(&(cl, ci))
        };

        let mut mark_pending = false;
        let mut curr_module = None;
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
                curr_module = prog.current_module;
            }
        }

        if mark_pending {
            let next_q = self.next_pending_in_level();
            let prog = self.progress_mut();
            prog.current_in_level = next_q;
        }

        if self.progress().current_in_level.is_none() {
            let module_idx = curr_module.unwrap_or(cw);
            if self.is_level_completed(module_idx, cl) {
                self.complete_level(module_idx, cl);

                if self.is_module_completed(module_idx) {
                    self.state = AppState::Summary;
                } else {
                    self.state = AppState::LevelSummary;
                }
            }
            if self.is_module_completed(module_idx) {
                self.complete_module(module_idx);
                self.state = AppState::Summary;
            }
        }

        self.sync_is_done();
        if correcta {
            self.update_input_prefill();
        }
        self.message = if correcta {
            "✅ ¡Correcto!".into()
        } else {
            match &grading_result {
                JudgeResult::WrongAnswer { test_index, .. } if *test_index == 0 => {
                    "❌ Incorrecto. Intenta de nuevo.".into()
                }
                _ => format_judge_message(&grading_result),
            }
        };
    }

    // ------------------------------------------------------------------
    // Remote judge (async via WASM spawn_local + mpsc channel)
    // ------------------------------------------------------------------

    #[cfg(target_arch = "wasm32")]
    fn start_remote_judge(&mut self, cw: usize, cl: usize, ci: usize, source: String) {
        let question = self.quiz.modules[cw].levels[cl].questions[ci].clone();
        let (tx, rx) = std::sync::mpsc::channel::<JudgeResult>();

        self.remote_judge_pending = Some(PendingRemoteJudge { cw, cl, ci });
        self.remote_judge_rx = Some(rx);
        self.message = "⏳ Evaluando en judge remoto...".into();

        wasm_bindgen_futures::spawn_local(async move {
            let result = grade_remote_question(&question, &source).await;
            let _ = tx.send(result);
        });
    }

    pub fn poll_remote_judge_result(&mut self) {
        let maybe_result = self
            .remote_judge_rx
            .as_ref()
            .and_then(|rx| rx.try_recv().ok());

        if let Some(result) = maybe_result {
            if let Some(pending) = self.remote_judge_pending.take() {
                self.apply_grading_result(pending.cw, pending.cl, pending.ci, result);
            }
            self.remote_judge_rx = None;
        }
    }

    pub fn is_remote_judge_pending(&self) -> bool {
        self.remote_judge_pending.is_some()
    }

    // ------------------------------------------------------------------
    // Navigation
    // ------------------------------------------------------------------

    pub fn saltar_pregunta(&mut self) {
        let (cw, cl, ci) = match self.current_position() {
            Some(pos) => pos,
            None => return,
        };

        {
            let q = &mut self.quiz.modules[cw].levels[cl].questions[ci];
            q.skips += 1;
            q.attempts += 1;
            q.saw_solution = false;
        }

        {
            let prog = self.progress_mut();
            if !prog.shown_this_round.contains(&(cl, ci)) {
                prog.shown_this_round.push((cl, ci));
            }
        }

        let next_q = self.next_pending_in_level();

        {
            let prog = self.progress_mut();
            prog.current_in_level = next_q;
            prog.input.clear();
        }

        self.finalize_level_or_module();

        self.update_input_prefill();
        self.message = "⏩ Pregunta saltada. La verás en la siguiente ronda.".to_string();
    }

    pub fn avanzar_a_siguiente_pregunta(&mut self) {
        let (cw, cl, ci) = match self.current_position() {
            Some(pos) => pos,
            None => return,
        };

        {
            let q = &mut self.quiz.modules[cw].levels[cl].questions[ci];
            q.saw_solution = true;
        }

        let next_q = self.next_pending_in_level();
        {
            let prog = self.progress_mut();
            prog.current_in_level = next_q;
            prog.input.clear();
            prog.show_solution = false;
        }

        self.finalize_level_or_module();
        self.update_input_prefill();
    }

    // ------------------------------------------------------------------
    // Test / debug helpers
    // ------------------------------------------------------------------

    pub fn complete_all_module(&mut self) {
        let wi = match self.progress().current_module {
            Some(w) => w,
            None => return,
        };
        let lang = self.selected_language.unwrap_or(Language::C);

        let mut ids = Vec::new();
        for lvl in &mut self.quiz.modules[wi].levels {
            for q in &mut lvl.questions {
                if q.language == lang {
                    if let Some(id) = q.mark_done_test() {
                        ids.push(id);
                    }
                }
            }
        }

        {
            let prog = self.progress_mut();
            for id in ids {
                prog.completed_ids.insert(id);
            }
        }

        self.complete_module(wi);
        self.sync_is_done();
        self.state = AppState::Summary;
    }

    pub fn complete_all_level(&mut self) {
        let wi = match self.progress().current_module {
            Some(w) => w,
            None => return,
        };
        let li = match self.progress().current_level {
            Some(l) => l,
            None => return,
        };
        let lang = self.selected_language.unwrap_or(Language::C);

        let mut ids = Vec::new();
        for q in &mut self.quiz.modules[wi].levels[li].questions {
            if q.language == lang {
                if let Some(id) = q.mark_done_test() {
                    ids.push(id);
                }
            }
        }

        {
            let prog = self.progress_mut();
            for id in ids {
                prog.completed_ids.insert(id);
            }
        }

        self.complete_level(wi, li);
        self.sync_is_done();
        self.state = AppState::LevelSummary;
    }

    // ------------------------------------------------------------------
    // Level navigation helpers
    // ------------------------------------------------------------------

    pub fn next_pending_in_module(&mut self) -> Option<(usize, usize)> {
        let progress = self.progress();
        let module_idx = progress.current_module?;
        let language = self.selected_language.unwrap_or(Language::C);

        let module = self.quiz.modules.get(module_idx)?;

        for (level_idx, level) in module.levels.iter().enumerate() {
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

    pub fn next_pending_in_level(&mut self) -> Option<usize> {
        let progress = self.progress();
        let module_idx = progress.current_module?;
        let level_idx = progress.current_level?;
        let language = self.selected_language.unwrap_or(Language::C);

        let module: Module = self.quiz.modules.get(module_idx)?.clone();
        let level = module.levels.get(level_idx)?;

        let shown = progress.shown_this_round.clone();

        for (q_idx, q) in level.questions.iter().enumerate() {
            if q.language == language {
                if let Some(id) = &q.id {
                    if !self.progress().completed_ids.contains(id)
                        && !shown.contains(&(level_idx, q_idx))
                    {
                        let progress = self.progress_mut();
                        progress.shown_this_round.push((level_idx, q_idx));
                        return Some(q_idx);
                    }
                }
            }
        }

        let hay_pendientes = level.questions.iter().enumerate().any(|(_q_idx, q)| {
            q.language == language
                && q.id
                    .as_ref()
                    .map(|id| !self.progress().completed_ids.contains(id))
                    .unwrap_or(false)
        });

        if hay_pendientes {
            {
                let progress = self.progress_mut();
                progress.round += 1;
                progress.shown_this_round.clear();
            }

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
