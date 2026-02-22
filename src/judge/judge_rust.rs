use crate::judge::judge_c::JudgeResult;
use crate::judge::judge_utils::simple_source_eq;
use crate::model::Question;

pub fn grade_rust_question(question: &Question, user_code: &str) -> JudgeResult {
    if simple_source_eq(user_code, &question.answer) {
        JudgeResult::Accepted
    } else {
        JudgeResult::WrongAnswer {
            test_index: 0,
            input: String::new(),
            expected: String::new(),
            received: String::new(),
            diff: "La respuesta Rust no coincide con la normalización esperada.".into(),
        }
    }
}
