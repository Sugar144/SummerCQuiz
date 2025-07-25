use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Language {
    C,
    Pseudocode,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Question {
    pub language: Language,
    pub week: usize,
    pub prompt: String,   // Pregunta
    pub answer: String,   // Respuesta
    pub hint: Option<String>,
    #[serde(default)]
    pub input_prefill: Option<String>,
    #[serde(default)]
    pub is_done: bool,
    #[serde(default)]
    pub saw_solution: bool,
    #[serde(default)]
    pub attempts: u32,
    #[serde(default)]
    pub fails: u32,
    #[serde(default)]
    pub skips: u32,
    // NUEVO
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AppState {
    LanguageSelect,
    Welcome,
    WeekMenu,
    Quiz,
    Summary,
    PendingUpdate,
}

// ¡Implementa Default!
impl Default for AppState {
    fn default() -> Self {
        AppState::Welcome
    }
}