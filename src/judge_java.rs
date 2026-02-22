use crate::judge_c::JudgeResult;
use crate::judge_utils::simple_source_eq;
use crate::model::Question;

pub fn grade_java_question(question: &Question, user_code: &str) -> JudgeResult {
    if simple_source_eq(user_code, &question.answer) {
        JudgeResult::Accepted
    } else {
        JudgeResult::WrongAnswer {
            test_index: 0,
            input: String::new(),
            expected: String::new(),
            received: String::new(),
            diff: "La respuesta Java no coincide con la normalización esperada.".into(),
        }
    }
}
