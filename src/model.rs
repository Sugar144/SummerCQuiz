use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum Language {
    C,
    Pseudocode,
    Kotlin,
    Java,
    Rust,
    Python,
    GitGithub,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GradingMode {
    Normalize,
    #[serde(alias = "judge_c_compile")]
    JudgeC,
    JudgePseudo,
    JudgeKotlin,
    JudgeJava,
    JudgeRust,
    JudgePython,
    #[serde(alias = "judge_remote")]
    JudgeRemote,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct JudgeTestCase {
    pub input: String,
    pub output: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Question {
    pub language: Language,
    #[serde(alias = "week")]
    pub module: usize,
    pub prompt: String,
    pub answer: String,
    pub hint: Option<String>,
    #[serde(skip)]
    pub number: usize,
    #[serde(default)]
    pub input_prefill: Option<String>,
    #[serde(default)]
    pub mode: Option<GradingMode>,
    #[serde(default)]
    pub tests: Vec<JudgeTestCase>,
    #[serde(default)]
    pub judge_harness: Option<String>,
    #[serde(default)]
    pub judge_endpoint: Option<String>,
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
pub struct Module {
    pub number: usize,
    pub explanation: String,
    pub levels: Vec<Level>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Quiz {
    #[serde(alias = "weeks")]
    pub modules: Vec<Module>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AppState {
    PendingUpdate,
    LanguageSelect,
    Welcome,
    ModuleMenu,
    LevelMenu,
    LevelTheory,
    Quiz,
    LevelSummary,
    Summary,
}

impl Default for AppState {
    fn default() -> Self {
        AppState::Welcome
    }
}

impl Question {
    pub fn uses_judge_c(&self) -> bool {
        matches!(self.mode, Some(GradingMode::JudgeC))
            || (self.language == Language::C && !self.tests.is_empty())
    }

    pub fn uses_judge_pseudo(&self) -> bool {
        matches!(self.mode, Some(GradingMode::JudgePseudo)) && !self.tests.is_empty()
    }

    pub fn reset_stats(&mut self) {
        self.is_done = false;
        self.attempts = 0;
        self.fails = 0;
        self.skips = 0;
        self.saw_solution = false;
    }

    pub fn mark_done_test(&mut self) -> Option<String> {
        self.is_done = true;
        self.saw_solution = false;
        self.attempts = 1;
        self.fails = 0;
        self.skips = 0;
        self.id.clone()
    }
}
