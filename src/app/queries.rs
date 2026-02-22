use super::*;

impl QuizApp {
    pub fn module(&self, n: usize) -> Option<&Module> {
        self.quiz.modules.iter().find(|w| w.number == n)
    }

    /// Devuelve referencia a un nivel concreto de una semana concreta
    pub fn level(&self, module: usize, level: usize) -> Option<&Level> {
        self.module(module)?
            .levels
            .iter()
            .find(|l| l.number == level)
    }

    /// Devuelve las preguntas de un nivel concreto de una semana concreta
    pub fn questions_for(&self, module: usize, level: usize) -> Option<&Vec<Question>> {
        self.level(module, level).map(|l| &l.questions)
    }

    /// Devuelve *mut* si necesitas modificar
    pub fn questions_for_mut(&mut self, module: usize, level: usize) -> Option<&mut Vec<Question>> {
        self.quiz
            .modules
            .iter_mut()
            .find(|w| w.number == module)?
            .levels
            .iter_mut()
            .find(|l| l.number == level)
            .map(|l| &mut l.questions)
    }

    // Si quieres aplanar todas las preguntas (muy útil para stats globales)
    pub fn all_questions(&self) -> Vec<&Question> {
        self.quiz
            .modules
            .iter()
            .flat_map(|w| w.levels.iter())
            .flat_map(|l| l.questions.iter())
            .collect()
    }

    pub fn all_question_ids(&self) -> HashSet<String> {
        self.quiz
            .modules
            .iter()
            .flat_map(|w| &w.levels)
            .flat_map(|l| &l.questions)
            .filter_map(|q| q.id.clone())
            .collect()
    }
}
