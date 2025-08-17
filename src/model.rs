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
    #[serde(skip)]
    pub number: usize,
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
pub struct Level {
    pub number: usize,
    pub explanation: std::collections::HashMap<Language, String>,
    pub questions: Vec<Question>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Week {
    pub number: usize,
    pub explanation: String,
    pub levels: Vec<Level>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Quiz {
    pub weeks: Vec<Week>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AppState {

    PendingUpdate,
    LanguageSelect,
    Welcome,
    WeekMenu,
    LevelMenu,
    LevelTheory,
    Quiz,
    LevelSummary,
    Summary,

}

// ¡Implementa Default!
impl Default for AppState {
    fn default() -> Self {
        AppState::Welcome
    }
}

impl Question {
    /// Reinicia los contadores y flags de esta pregunta.
    pub fn reset_stats(&mut self) {
        self.is_done = false;
        self.attempts = 0;
        self.fails = 0;
        self.skips = 0;
        self.saw_solution = false;
    }

    /// Marca esta pregunta como completada (modo TEST), resetea estadísticas
    /// y devuelve su `id` clonada si la tuviera.
    pub fn mark_done_test(&mut self) -> Option<String> {
        self.is_done = true;
        self.saw_solution = false;
        self.attempts = 1;
        self.fails = 0;
        self.skips = 0;
        self.id.clone()
    }
}
