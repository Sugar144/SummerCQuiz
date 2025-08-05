// src/data.rs

use crate::model::Quiz;
use serde_yaml;

/// Carga el banco de preguntas desde el YAML embebido
pub fn read_questions_embedded() -> Quiz {
    // Ajusta la ruta si pones tu yaml en otra carpeta
    let file_content = include_str!("data/quiz_questions_v2.yaml");
    serde_yaml::from_str(file_content).expect("No se pudo parsear el banco de preguntas YAML")
}
