use super::*;

impl QuizApp {
    pub fn week(&self, n: usize) -> Option<&Week> {
        self.quiz.weeks.iter().find(|w| w.number == n)
    }

    /// Devuelve referencia a un nivel concreto de una semana concreta
    pub fn level(&self, week: usize, level: usize) -> Option<&Level> {
        self.week(week)?.levels.iter().find(|l| l.number == level)
    }

    /// Devuelve las preguntas de un nivel concreto de una semana concreta
    pub fn questions_for(&self, week: usize, level: usize) -> Option<&Vec<Question>> {
        self.level(week, level).map(|l| &l.questions)
    }

    /// Devuelve *mut* si necesitas modificar
    pub fn questions_for_mut(&mut self, week: usize, level: usize) -> Option<&mut Vec<Question>> {
        self.quiz.weeks.iter_mut()
            .find(|w| w.number == week)?
            .levels.iter_mut()
            .find(|l| l.number == level)
            .map(|l| &mut l.questions)
    }

    // Si quieres aplanar todas las preguntas (muy útil para stats globales)
    pub fn all_questions(&self) -> Vec<&Question> {
        self.quiz.weeks
            .iter()
            .flat_map(|w| w.levels.iter())
            .flat_map(|l| l.questions.iter())
            .collect()
    }

    pub fn all_question_ids(&self) -> HashSet<String> {
        self.quiz
            .weeks
            .iter()
            .flat_map(|w| &w.levels)
            .flat_map(|l| &l.questions)
            .filter_map(|q| q.id.clone())
            .collect()
    }
}

