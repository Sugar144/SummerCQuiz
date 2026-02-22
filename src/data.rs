use crate::model::{Language, Quiz};

pub fn read_questions_for_language(language: Language) -> Quiz {
    let file_content = match language {
        Language::C | Language::Pseudocode | Language::GitGithub => {
            include_str!("data/quiz_questions_modes.yaml")
        }
        Language::Kotlin => include_str!("data/quiz_questions_kotlin.yaml"),
        Language::Java => include_str!("data/quiz_questions_java.yaml"),
        Language::Rust => include_str!("data/quiz_questions_rust.yaml"),
        Language::Python => include_str!("data/quiz_questions_python.yaml"),
    };

    serde_yaml::from_str(file_content).expect("No se pudo parsear el banco de preguntas YAML")
}
