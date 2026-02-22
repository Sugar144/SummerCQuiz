mod helpers;
pub mod layout;
pub mod views;

use crate::app::QuizApp;
use crate::model::AppState;
use eframe::{APP_KEY, App, Frame, set_value};
use egui::Context;
use layout::{bottom_panel, top_panel};

impl App for QuizApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        // BOTÓN SUPERIOR DE REINICIAR y CAMBIAR LENGUAJE (solo visible durante el quiz y resumen)
        if matches!(self.state, AppState::Quiz | AppState::Summary) {
            top_panel(self, ctx, true);

            // BOTÓN SUPERIOR CAMBIAR LENGUAJE (solo visible durante el quiz, resumen y welcome)
        } else if matches!(
            self.state,
            AppState::Quiz | AppState::Summary | AppState::Welcome
        ) {
            top_panel(self, ctx, false);
        }

        // PANEL INFERIOR TEMA OSCURO O CLARO
        bottom_panel(ctx);

        // Dispatch por estado a las funciones en views.rs
        match self.state {
            AppState::PendingUpdate => views::pending::ui_pending_update(self, ctx),
            AppState::LanguageSelect => views::language::ui_language_select(self, ctx),
            AppState::Welcome => views::welcome::ui_welcome(self, ctx),
            AppState::ModuleMenu => views::module_menu::ui_module_menu(self, ctx),
            AppState::LevelMenu => views::level_menu::ui_level_menu(self, ctx),
            AppState::Quiz => views::quiz::ui_quiz(self, ctx),
            AppState::Summary => views::summary::ui_summary_view(self, ctx),
            AppState::LevelSummary => views::level_summary::ui_level_summary(self, ctx),
            AppState::LevelTheory => views::level_theory::ui_level_theory(self, ctx),
        }

        if self.confirm_reset {
            self.confirm_reset(ctx);
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        set_value(storage, APP_KEY, self);
    }
}
