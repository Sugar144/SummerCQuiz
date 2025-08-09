// src/view_models.rs

#[derive(Clone, Debug)]
pub struct WeekInfo {
    pub idx: usize,        // índice 0-based en quiz.weeks
    pub number: usize,     // número "humano" (1,2,3…)
    pub unlocked: bool,
    pub completed: bool,
    pub new_count: usize,  // cuántas nuevas pendientes
}

#[derive(Clone, Debug)]
pub struct LevelInfo {
    pub idx: usize,
    pub number: usize,
    pub unlocked: bool,
    pub completed: bool,
    pub new_count: usize,
}

#[derive(Clone, Debug)]
pub struct QuestionRow {
    pub level_number: usize,
    pub question_index_1based: usize,
    pub attempts: u32,
    pub fails: u32,
    pub skips: u32,
    pub saw_solution: bool,
    pub done: bool,
}

impl WeekInfo {
    pub fn label(&self) -> String {
        if self.completed && self.new_count == 0 {
            format!("Semana {} ✅", self.number)
        } else if self.unlocked {
            if self.new_count > 0 {
                format!("Semana {} 🔓 ({} nuevas)", self.number, self.new_count)
            } else {
                format!("Semana {} 🔓", self.number)
            }
        } else {
            format!("Semana {} 🔒", self.number)
        }
    }
}

impl LevelInfo {
    pub fn label(&self) -> String {
        if self.completed && self.new_count == 0 {
            format!("Nivel {} ✅", self.number)
        } else if self.unlocked {
            if self.new_count > 0 {
                format!("Nivel {} 🔓 ({} nuevas)", self.number, self.new_count)
            } else {
                format!("Nivel {} 🔓", self.number)
            }
        } else {
            format!("Nivel {} 🔒", self.number)
        }
    }
}
