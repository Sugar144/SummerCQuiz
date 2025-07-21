use eframe::egui;
use egui::Visuals;
use serde::{Serialize, Deserialize };

use once_cell::sync::Lazy;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;
use syntect::easy::HighlightLines;
use syntect::util::LinesWithEndings;
use syntect::dumps::from_binary;

use reqwest::blocking::Client;
use reqwest::header::USER_AGENT;



static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| {
    from_binary(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/syntaxes.packdump")))
});
static THEME_SET: Lazy<ThemeSet> = Lazy::new(|| {
    from_binary(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/themes.packdump")))
});


#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
enum Language {
    C,
    Pseudocode
}

#[derive(Serialize, Deserialize)]
struct Question {
    language: Language,
    week: usize,
    prompt: String,  // Preguntas
    answer: String,  // Respuestas
    hint:Option<String>,
    #[serde(default)]
    input_prefill: Option<String>,
    #[serde(default)]
    is_done: bool,   // true si respondida correctamente
    #[serde(default)]
    saw_solution: bool,
    #[serde(default)]
    attempts: u32,   // intentos totales (aciertos+fallos+saltos)
    #[serde(default)]
    fails: u32,      // respuestas incorrectas
    #[serde(default)]
    skips: u32,      // veces saltadas
}

enum AppState {
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

#[derive(Serialize, Deserialize)]
struct QuizApp {
    questions: Vec<Question>,
    selected_language: Option<Language>,
    current_week: Option<usize>,
    unlocked_weeks: Vec<usize>,
    max_unlocked_week: usize,
    current_in_week: Option<usize>,
    input: String,
    message: String,
    finished: bool,
    round: u32,
    shown_this_round: Vec<usize>,
    show_solution: bool,
    #[serde(skip)]
    state: AppState,
    #[serde(skip)]
    has_update: Option<String>,
    #[serde(skip)]
    confirm_reset: bool,
    #[serde(skip)]
    update_thread_launched:bool
}

fn progress_filename(language: Language) -> &'static str {
    match language {
        Language::C => "quiz_progress_c.json",
        Language::Pseudocode => "quiz_progress_pseudocode.json",
    }
}

impl QuizApp {
    pub fn new() -> Self {
        let mut questions = read_questions_embedded();

        for q in &mut questions {
            q.is_done = false;
            q.attempts = 0;
            q.fails = 0;
            q.skips = 0;
            q.saw_solution = false;
        }

        // Primero creas el struct (puedes llamarlo quiz_app, o self si lo prefieres)
        let mut quiz_app = Self {
            questions,
            selected_language: None,
            current_week: None,
            unlocked_weeks: vec![1],
            max_unlocked_week: 1,
            current_in_week: None,
            input: String::new(),
            message: String::new(),
            finished: false,
            round: 1,
            shown_this_round: vec![],
            show_solution: false,
            state: AppState::LanguageSelect,
            has_update: None,
            confirm_reset: false,
            update_thread_launched: false,
        };

        // Luego chequeas si hay actualización y pones el mensaje
        let signal_path = std::path::Path::new(".update_success");
        if signal_path.exists() {
            // ¡La versión que importa es la que corre AHORA!
            quiz_app.message = format!(
                "¡Actualización a versión {} completada!",
                env!("CARGO_PKG_VERSION")
            );
            let _ = std::fs::remove_file(signal_path);
        }

        // Devuelves el struct ya inicializado
        quiz_app
    }


    pub fn new_for_language(language: Language) -> Self {
        let mut questions = read_questions_embedded()
            .into_iter()
            .filter(|q| q.language == language)
            .collect::<Vec<_>>();

        // Forzar estado limpio al crear nuevo quiz
        for q in &mut questions {
            q.is_done = false;
            q.attempts = 0;
            q.fails = 0;
            q.skips = 0;
            q.saw_solution = false;
        }

        let first_week = questions.iter()
            .map(|q| q.week)
            .min()
            .unwrap_or(1);

        let current_in_week = questions
            .iter()
            .position(|q| q.week == first_week && !q.is_done);

        Self {
            questions,
            selected_language: Some(language),
            current_week: Some(first_week),
            unlocked_weeks: vec![1],
            max_unlocked_week: 1,
            current_in_week,
            input: String::new(),
            message: String::new(),
            finished: false,
            round: 1,
            shown_this_round: vec![],
            show_solution: false,
            state: AppState::Welcome,
            has_update: None,
            confirm_reset: false,
            update_thread_launched: false,
        }
    }
    pub fn save_progress(&self) {
        if let Some(lang) = self.selected_language {
            let filename = progress_filename(lang);
            let json = serde_json::to_string(self).unwrap();
            std::fs::write(filename, json).unwrap();
        }
    }

    pub fn load_progress(language: Language) -> Option<Self> {
        let filename = progress_filename(language);
        if let Ok(json) = std::fs::read_to_string(filename) {
            serde_json::from_str(&json).ok()
        } else {
            None
        }
    }

    pub fn delete_progress(language: Language) {
        let filename = match language {
            Language::C => "quiz_progress_c.json",
            Language::Pseudocode => "quiz_progress_pseudocode.json",
        };
        let _ = std::fs::remove_file(filename);
    }

    pub fn has_saved_progress(language: Language) -> bool {
        let filename = match language {
            Language::C => "quiz_progress_c.json",
            Language::Pseudocode => "quiz_progress_pseudocode.json",
        };
        std::fs::metadata(filename).is_ok()
    }

    fn select_week(&mut self, week: usize) {

        // Si la semana seleccionada es mayor que el max actual, actualiza el máximo
        if week > self.max_unlocked_week {
            self.max_unlocked_week = week;
            self.recalculate_unlocked_weeks();
        } else {
            // Asegúrate de que la semana seleccionada está en el vector de semanas desbloqueadas (por si acaso)
            if !self.unlocked_weeks.contains(&week) {
                self.unlocked_weeks.push(week);
                self.unlocked_weeks.sort();
            }
        }

        self.current_week = Some(week);
        let language = self.selected_language.unwrap_or(Language::C);

        // ¿Todas las preguntas de la semana están is_done? Resetea solo las de esa semana
        let week_done = self.questions
            .iter()
            .filter(|q| q.week == week && q.language == language)
            .all(|q| q.is_done);

        if week_done {
            for q in self.questions.iter_mut().filter(|q| q.week == week && q.language == language) {
                q.is_done = false;
                q.fails = 0;
                q.attempts = 0;
                q.skips = 0;
                q.saw_solution = false;
            }
        }

        // Selecciona la primera pendiente
        self.current_in_week = self.questions
            .iter()
            .enumerate()
            .find(|(_, q)| q.week == week && q.language == language && !q.is_done)
            .map(|(idx, _)| idx);
        self.round = 1;
        self.shown_this_round.clear();
    }



    // Al completar una semana, desbloquea la siguiente
    pub fn complete_week(&mut self, week: usize) {
        let language = self.selected_language.unwrap_or(Language::C);
        if self.is_week_completed(week) {
            let next_week = week + 1;
            if self.questions.iter().any(|q| q.week == next_week && q.language == language) {
                if next_week > self.max_unlocked_week {
                    self.max_unlocked_week = next_week;
                }
            }
            self.recalculate_unlocked_weeks(); // <-- ¡SIEMPRE LLAMAR AQUÍ!
        }
    }



    // Cambia el is_week_unlocked:
    pub fn is_week_unlocked(&self, week: usize) -> bool {
        self.unlocked_weeks.contains(&week)
    }


    // Una semana está completa si todas sus preguntas están respondidas correctamente
    pub fn is_week_completed(&self, week: usize) -> bool {
        let language = self.selected_language.unwrap_or(Language::C);
        let questions_in_week: Vec<_> = self.questions
            .iter()
            .filter(|q| q.week == week && q.language == language)
            .collect();

        if questions_in_week.is_empty() {
            // Si no hay preguntas para esa semana, NO puede estar completada
            false
        } else {
            questions_in_week.iter().all(|q| q.is_done)
        }
    }


    fn recalculate_unlocked_weeks(&mut self) {
        self.unlocked_weeks.clear();
        for week in 1..=self.max_unlocked_week {
            self.unlocked_weeks.push(week);
        }
    }





    /// Busca la próxima pregunta pendiente, o None si no quedan
    fn next_pending_in_week(&mut self) -> Option<usize> {
        if let Some(week) = self.current_week {
            for (idx, q) in self.questions.iter().enumerate() {
                if q.week == week && !q.is_done && !self.shown_this_round.contains(&idx) {
                    self.shown_this_round.push(idx);
                    return Some(idx);
                }
            }
            // Si ya se han mostrado todas las pendientes, empieza nueva ronda
            if self.questions.iter().any(|q| q.week == week && !q.is_done) {
                self.round += 1;
                self.shown_this_round.clear();
                for (idx, q) in self.questions.iter().enumerate() {
                    if q.week == week && !q.is_done {
                        self.shown_this_round.push(idx);
                        return Some(idx);
                    }
                }
            }
        }
        None
    }

    fn update_input_prefill(&mut self) {
        if let Some(idx) = self.current_in_week {
            if let Some(prefill) = &self.questions[idx].input_prefill {
                self.input = prefill.clone();
            } else {
                self.input.clear();
            }
        }
    }

    /// Borra progreso y reinicia el quiz para el lenguaje actual
    fn reset_progress(&mut self) {
        if let Some(language) = self.selected_language {
            Self::delete_progress(language);
            *self = QuizApp::new_for_language(language);
            self.state = AppState::Quiz;
            self.update_input_prefill();
            self.confirm_reset = false;
            self.message.clear();
        }
    }

    fn confirm_reset(&mut self, ctx: &egui::Context) {
        egui::Window::new("Confirmar reinicio")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("¿Seguro que quieres borrar todo tu progreso? ¡Esta acción no se puede deshacer!");
                ui.horizontal(|ui| {
                    if ui.button("Sí, borrar").clicked() {
                        self.reset_progress();
                    }
                    if ui.button("No").clicked() {
                        self.confirm_reset = false;
                    }
                });
            });
    }


}

fn check_latest_release() -> Result<Option<String>, Box<dyn std::error::Error>> {
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner("Sugar144")
        .repo_name("SummerCQuiz")
        .build()?
        .fetch()?;

    if let Some(release) = releases.first() {
        let latest_version = release.version.clone();
        let current_version = env!("CARGO_PKG_VERSION").to_string();
        if latest_version != current_version {
            return Ok(Some(latest_version));
        }
    }
    Ok(None)
}

fn descargar_binario_nuevo() -> Result<(), Box<dyn std::error::Error>> {
    use self_update::backends::github::ReleaseList;
    use std::fs::File;

    let releases = ReleaseList::configure()
        .repo_owner("Sugar144")
        .repo_name("SummerCQuiz")
        .build()?
        .fetch()?;

    let release = releases.first().expect("No hay releases");

    let (asset_name, local_name) = if cfg!(windows) {
        ("SummerQuiz.exe", "SummerQuiz_new.exe")
    } else {
        ("SummerQuiz", "SummerQuiz_new")
    };

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .expect("No se encontró asset para el sistema actual");

    // CORREGIDO: Usa Client y añade User-Agent
    let client = Client::new();
    let mut resp = client
        .get(&asset.download_url)
        .header(USER_AGENT, "SummerQuiz-Updater/1.0")
        .header(reqwest::header::ACCEPT, "application/octet-stream")
        .send()?;
    let mut out = File::create(local_name)?;
    resp.copy_to(&mut out)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(local_name, perms)?;
    }

    Ok(())
}





fn normalize_code(input: &str) -> String {
    let mut code = String::new();
    let mut in_block_comment = false;
    for line in input.lines() {
        let mut line = line;
        // Eliminar comentarios de bloque que empiezan en esta línea
        if !in_block_comment {
            if let Some(start) = line.find("/*") {
                in_block_comment = true;
                line = &line[..start];
            }
        }
        // Eliminar comentarios de línea //
        if !in_block_comment {
            if let Some(start) = line.find("//") {
                line = &line[..start];
            }
        }
        // Si estamos dentro de un bloque /* ... */
        if in_block_comment {
            if let Some(end) = line.find("*/") {
                in_block_comment = false;
                line = &line[(end + 2)..];
            } else {
                continue; // línea completamente comentada
            }
        }
        // Añadir línea si queda algo útil
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            code.push_str(trimmed);
        }
    }
    code.replace(char::is_whitespace, "")
}



fn read_questions_embedded() -> Vec<Question> {
    // Ruta relativa al archivo fuente donde pongas este código.
    // Si lo tienes en data/, usa así:
    let file_content = include_str!("data/quiz_questions.yaml");
    serde_yaml::from_str(file_content).expect("No se pudo parsear el banco de preguntas YAML")
}



pub fn show_highlighted_c_code(ui: &mut egui::Ui, code: &str, theme_name: &str, panel_width: f32,
    max_input_height: f32, min_lines: usize, ) {

    let ss = &*SYNTAX_SET;
    let ts = &*THEME_SET;
    let syntax = ss
        .find_syntax_by_extension("c")
        .expect("No se encontró sintaxis para C");
    let mut h = HighlightLines::new(syntax, &ts.themes[theme_name]);

    let font_id = egui::TextStyle::Monospace.resolve(ui.style());
    let line_height = ui.fonts(|f| f.row_height(&font_id));
    let code_lines = code.lines().count();
    let lines = code_lines.max(min_lines);
    let needed_height = (lines as f32 * line_height).min(max_input_height);

    egui::Frame::default()
        .fill(ui.visuals().extreme_bg_color)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(needed_height)
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.set_width(panel_width);
                    for (i,line) in LinesWithEndings::from(code).enumerate() {
                        let regions = h.highlight_line(line, ss).unwrap();
                        ui.horizontal(|ui| {
                            ui.add_space(3.0);
                            ui.spacing_mut().item_spacing.x = 0.0;

                            // Añade el número de línea con color/gris
                            ui.colored_label(
                                egui::Color32::DARK_GRAY,
                                egui::RichText::new(format!("{:>2} ", i + 1)).monospace(),
                            );

                            for (style, text) in regions {
                                let color = egui::Color32::from_rgb(
                                    style.foreground.r,
                                    style.foreground.g,
                                    style.foreground.b,
                                );
                                ui.colored_label(color, egui::RichText::new(text).monospace());
                            }
                        });
                    }
                });
        });
}







impl eframe::App for QuizApp {

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // BOTÓN SUPERIOR DE REINICIAR (solo visible durante el quiz y resumen)
        if matches!(self.state, AppState::Quiz | AppState::Summary) {
            egui::TopBottomPanel::top("menu_panel").show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if ui.button("🔄 Borrar progreso y reiniciar").clicked() {
                        self.confirm_reset = true;
                    }

                    if ui.button("Cambiar lenguaje").clicked() {
                        self.save_progress();
                        self.state = AppState::LanguageSelect;

                    }

                    if self.confirm_reset {
                        self.confirm_reset(ctx);
                    }


                });
            });
        }else if matches!(self.state, AppState::Quiz | AppState::Summary | AppState::Welcome) {
            egui::TopBottomPanel::top("menu_panel").show(ctx, |ui| {
                ui.horizontal_centered(|ui| {

                    if ui.button("Cambiar lenguaje").clicked() {
                        self.save_progress();
                        self.state = AppState::LanguageSelect;

                    }

                });
            });
        }

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            // ----------- BOTONES DE TEMA -----------
            ui.with_layout(
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    if ui.button("🌙 Modo oscuro").clicked() {
                        ctx.set_visuals(Visuals::dark());
                    }
                    if ui.button("☀Modo claro").clicked() {
                        ctx.set_visuals(Visuals::light());
                    }
                }
            );

        });



        match self.state {
            AppState::PendingUpdate => {
                // Aquí pones el panel
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(60.0);
                        ui.label(
                            egui::RichText::new(&self.message)
                                .heading()
                                .color(egui::Color32::YELLOW)
                                .strong()
                        );
                        ui.add_space(20.0);
                        ui.add(egui::Spinner::new());
                        ui.add_space(60.0);
                    });
                });

                // Lanzas el thread SOLO la primera vez
                if !self.update_thread_launched {
                    self.update_thread_launched = true;
                    let updater = if cfg!(windows) {
                        "SummerQuizUpdater.exe".to_string()
                    } else {
                        "./SummerQuizUpdater".to_string()
                    };
                    std::thread::spawn(move || {
                        let res = descargar_binario_nuevo();
                        match res {
                            Ok(()) => {
                                std::thread::sleep(std::time::Duration::from_secs(2));
                                std::process::Command::new(&updater)
                                    .spawn()
                                    .expect("No se pudo lanzar el updater");
                                std::process::exit(0);
                            }
                            Err(e) => {
                                eprintln!("Error al descargar actualización: {e}");
                            }
                        }
                    });
                }
                // No haces nada más, la UI ya ha cambiado el mensaje
            }
            // ----------- BIENVENIDA -----------
            AppState::LanguageSelect => {
                self.message.clear();
                egui::CentralPanel::default().show(ctx, |ui| {
                    // Calcula el espacio vertical extra
                    let total_height = 300.0; // tu contenido: aprox. heading + botones, etc.
                    let extra_space = (ui.available_height() - total_height).max(0.0) / 2.0;
                    ui.add_space(extra_space);

                    // Máximo ancho que quieres permitir (por si pantalla ultra-wide)
                    let max_width = 540.0;
                    let content_width = ui.available_width().min(max_width);

                    // Centrar
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        // Opcional: añade un panel contenedor para poner un borde, margen, color, etc.
                        egui::Frame::default()
                            .fill(ui.visuals().window_fill())
                            .inner_margin(egui::Margin::symmetric(16, 16))
                            .show(ui, |ui| {
                                ui.set_width(content_width);

                                ui.heading("👋 ¡Bienvenido a SummerQuiz!");
                                ui.add_space(18.0);
                                ui.label("Selecciona un lenguaje");
                                ui.add_space(18.0);

                                let button_width = (content_width - 40.0) / 2.0;
                                let button_width = button_width.clamp(120.0, 280.0); // Nunca demasiado pequeño ni enorme

                                // Botones centrados y adaptativos
                                ui.vertical_centered(|ui| {

                                    let c = ui.add_sized([button_width, 40.0], egui::Button::new("Lenguaje C"));
                                    let pseudocode = ui.add_sized([button_width, 40.0], egui::Button::new("Pseudocódigo"));

                                    let mut selected: Option<Language> = None;
                                    if c.clicked() {
                                        selected = Some(Language::C);
                                    }
                                    if pseudocode.clicked() {
                                        selected = Some(Language::Pseudocode);
                                    }
                                    if let Some(lang) = selected {
                                        self.selected_language = Some(lang);
                                        if let Some(progress) = QuizApp::load_progress(lang) {
                                            *self = progress;
                                        } else {
                                            *self = QuizApp::new_for_language(lang);
                                        }
                                        self.state = AppState::Welcome;
                                        self.message.clear();
                                    }

                                });

                                ui.add_space(16.0);

                                if self.has_update.is_none() {
                                    self.has_update = match check_latest_release() {
                                        Ok(Some(new_ver)) => Some(new_ver),
                                        Ok(None) => Some("".to_string()),
                                        Err(_) => Some("".to_string()),
                                    };
                                }

                                if let Some(ver) = &self.has_update {
                                    if !ver.is_empty() {
                                        let update = ui.add_sized(
                                            [button_width, 40.0],
                                            egui::Button::new(format!("⬇ Actualizar a {ver}"))
                                        );

                                        if update.clicked() {
                                            self.message = "Iniciando actualización…".to_string();
                                            self.state = AppState::PendingUpdate; // Cambia el estado
                                            ctx.request_repaint();
                                            // NO hagas nada más aquí, la lógica se moverá al ciclo principal.
                                        }

                                        ui.add_space(10.0);
                                    }
                                }
                                ui.add_space(12.0);

                                if !self.message.is_empty() {
                                    ui.add_space(10.0);
                                    ui.label(
                                        egui::RichText::new(&self.message)
                                            .color(egui::Color32::YELLOW)
                                            .heading()
                                            .strong()
                                    );
                                }
                            });
                    });
                });
            }



            // ----------- BIENVENIDA -----------
            AppState::Welcome => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let max_width = 540.0;
                    let available_w = ui.available_width();
                    let content_width = available_w.min(max_width);

                    // Estima altura del bloque de contenido
                    let estimated_height = 230.0;
                    let vertical_space = ((ui.available_height() - estimated_height) / 2.0).max(0.0);

                    ui.add_space(vertical_space / 2.0);

                    ui.horizontal_centered(|ui| {
                        egui::Frame::default()
                            .fill(ui.visuals().window_fill())
                            .inner_margin(egui::Margin::symmetric(16, 16))
                            .show(ui, |ui| {
                                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                    ui.heading("¿Qué deseas hacer?");
                                    ui.add_space(18.0);

                                    let hay_guardado = Self::has_saved_progress(self.selected_language.unwrap());
                                    let button_w = (content_width * 0.9).clamp(120.0, 400.0);
                                    let button_h = 36.0;

                                    let continuar_btn = if hay_guardado {
                                        Some(ui.add_sized([button_w, button_h], egui::Button::new("▶ Continuar donde lo dejé")))
                                    } else {
                                        None
                                    };

                                    let empezar_btn = ui.add_sized([button_w, button_h], egui::Button::new("🔄 Empezar de 0"));

                                    let menu_semanal_btn = ui.add_sized([button_w, button_h], egui::Button::new("📅 Seleccionar Semana"));

                                    let salir_btn = ui.add_sized([button_w, button_h], egui::Button::new("❌ Salir"));


                                    // --- Manejar clicks ---
                                    if let Some(btn) = continuar_btn {
                                        if btn.clicked() {
                                            if self.current_week.is_none() || self.current_in_week.is_none() {
                                                if let Some(first_week) = self.questions.iter().filter(|q| !q.is_done).map(|q| q.week).min() {
                                                    self.select_week(first_week);
                                                    self.update_input_prefill();
                                                } else {
                                                    let first_week = self.questions.iter().map(|q| q.week).min().unwrap_or(1);
                                                    self.select_week(first_week);
                                                    self.update_input_prefill();
                                                }
                                            }
                                            self.state = AppState::Quiz;
                                            self.finished = false;
                                            self.input.clear();
                                            self.message.clear();
                                        }
                                    }

                                    if empezar_btn.clicked() && hay_guardado {
                                        self.confirm_reset = true;
                                    } else if empezar_btn.clicked() {
                                        self.reset_progress();
                                        self.message.clear();  // ← AÑADE AQUÍ
                                    }

                                    if menu_semanal_btn.clicked() {
                                        self.state = AppState::WeekMenu;
                                    }

                                    if salir_btn.clicked() {
                                        std::process::exit(0);
                                    }

                                    if self.confirm_reset {
                                        self.confirm_reset(ctx);
                                    }

                                });
                            });
                    });

                    ui.add_space(vertical_space);
                });
            }

            AppState::WeekMenu => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let max_width = 540.0;
                    let content_width = ui.available_width().min(max_width);
                    let button_w = (content_width * 0.9).clamp(140.0, 400.0);
                    let button_h = 36.0;

                    // Calcular nº de semanas para estimar altura
                    let weeks_count = self.questions
                        .iter()
                        .filter(|q| q.language == self.selected_language.unwrap_or(Language::C))
                        .map(|q| q.week)
                        .collect::<std::collections::HashSet<_>>()
                        .len();

                    let estimated_height = 80 + (button_h as usize + 8) * (weeks_count + 1);
                    let vertical_space = ((ui.available_height() - estimated_height as f32) / 2.0).max(0.0);

                    ui.add_space(vertical_space / 2.0);

                    // Este bloque centra el Frame horizontalmente
                    ui.horizontal_centered(|ui| {
                        egui::Frame::default()
                            .fill(ui.visuals().window_fill())
                            .inner_margin(egui::Margin::symmetric(24, 16))
                            .show(ui, |ui| {
                                // No pongas set_width aquí
                                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                    ui.heading("Selecciona una semana");
                                    ui.add_space(20.0);

                                    let language = self.selected_language.unwrap_or(Language::C);

                                    let mut weeks_with_questions: Vec<usize> = self.questions
                                        .iter()
                                        .filter(|q| q.language == language)
                                        .map(|q| q.week)
                                        .collect();

                                    weeks_with_questions.sort_unstable();
                                    weeks_with_questions.dedup();

                                    let mut buttons = vec![];
                                    for &week in &weeks_with_questions {
                                        let unlocked = self.is_week_unlocked(week);
                                        let completed = self.is_week_completed(week);
                                        let label = if completed {
                                            format!("Semana {} ✅", week)
                                        } else if unlocked {
                                            format!("Semana {} 🔓", week)
                                        } else {
                                            format!("Semana {} 🔒", week)
                                        };

                                        let button = ui.add_sized(
                                            [button_w, button_h],
                                            egui::Button::new(label)
                                        ).on_hover_text("Pulsa para acceder a esta semana");
                                        buttons.push((week, button, unlocked));
                                        ui.add_space(8.0);
                                    }

                                    ui.add_space(16.0);

                                    let volver_btn = ui.add_sized([button_w, button_h], egui::Button::new("Volver al menú principal"));

                                    // --- Gestión de clicks ---
                                    for (week, button, unlocked) in buttons {
                                        if button.clicked() && unlocked {
                                            self.select_week(week);
                                            self.state = AppState::Quiz;
                                            self.update_input_prefill();
                                            self.message.clear();
                                            self.save_progress();
                                        }
                                    }
                                    if volver_btn.clicked() {
                                        self.state = AppState::Welcome;
                                        self.message.clear();
                                    }
                                });
                            });
                    });

                    ui.add_space(vertical_space);
                });
            }





            // ----------- QUIZ NORMAL -----------
            AppState::Quiz => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let max_width = 600.0;
                    let panel_width = (ui.available_width() * 0.97).min(max_width);
                    let total_height = 150.0 + 245.0 + 48.0 + 48.0 + 24.0;
                    let extra_space = (ui.available_height() - total_height).max(0.0) / 2.0;
                    ui.add_space(extra_space);

                    egui::Frame::default()
                        .fill(ui.visuals().window_fill())
                        .inner_margin(egui::Margin::symmetric(120, 20))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                if let Some(idx) = self.current_in_week {
                                    ui.heading(format!("🌀 Ronda {}", self.round));
                                    // Prompt con scroll fijo
                                    let prompt_max_height = 250.0;
                                    let prompt_min_lines = 4.0;
                                    let font_id = egui::TextStyle::Body.resolve(ui.style());
                                    let line_height = ui.fonts(|f| f.row_height(&font_id));
                                    let prompt_min_height = prompt_min_lines * line_height;
                                    let prompt_text = &self.questions[idx].prompt;
                                    let galley = ui.fonts(|fonts| {
                                        fonts.layout(
                                            prompt_text.to_owned(),
                                            font_id.clone(),
                                            egui::Color32::WHITE,
                                            panel_width,
                                        )
                                    });
                                    let needed_height = galley.size().y.max(prompt_min_height).min(prompt_max_height);
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(panel_width, needed_height),
                                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                                        |ui| {
                                            egui::ScrollArea::vertical()
                                                .max_height(prompt_max_height)
                                                .show(ui, |ui| {
                                                    ui.with_layout(
                                                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                                                        |ui| {
                                                            ui.label(prompt_text);
                                                        }
                                                    );
                                                });
                                        }
                                    );



                                    ui.add_space(5.0);

                                    let max_input_height = 245.0;

                                    let answer_box = |ui: &mut egui::Ui, content: &mut String, editable: bool| {
                                        egui::ScrollArea::vertical()
                                            .max_height(max_input_height)
                                            .auto_shrink([false; 2])
                                            .show(ui, |ui| {
                                                let text_edit = egui::TextEdit::multiline(content)
                                                    .desired_width(panel_width)
                                                    .desired_rows(16)
                                                    .font(egui::TextStyle::Monospace)
                                                    .interactive(editable);
                                                ui.add(text_edit);
                                            });
                                    };


                                    if self.questions[idx].fails >= 1 {

                                        // Si no se ha mostrado la solución todavía
                                        if !self.show_solution {
                                            ui.horizontal(|ui| {
                                                if ui.button("Solución").clicked() {
                                                    self.show_solution = true;
                                                }
                                            });
                                            answer_box(ui, &mut self.input, true);
                                        } else {
                                            ui.horizontal(|ui | {
                                                if ui.button("Siguiente pregunta").clicked() {
                                                    self.questions[idx].saw_solution = true;
                                                    self.show_solution = false; // Reset
                                                    self.input.clear();
                                                    self.current_in_week = self.next_pending_in_week();

                                                    self.update_input_prefill();

                                                    // Solo pasas de semana si toto esta acertado
                                                    if self.current_in_week.is_none() && self.is_week_completed(self.current_week.unwrap_or(1)) {
                                                        let week = self.current_week.unwrap_or(1);
                                                        self.complete_week(week);
                                                        self.state = AppState::Summary;
                                                    }
                                                    self.save_progress();
                                                }
                                            });

                                            let max_input_height = 245.0;
                                            let min_lines = 16;

                                            // ---------- AQUÍ CAMBIA EL BLOQUE ----------
                                            if self.questions[idx].language == Language::C {
                                                ui.push_id("highlighted_solution", |ui| {
                                                    show_highlighted_c_code(ui, &self.questions[idx].answer, "base16-onedark", panel_width, max_input_height, min_lines);
                                                });
                                            } else {
                                                let answer_string = self.questions[idx].answer.clone();
                                                answer_box(ui, &mut answer_string.clone(), false);
                                            }
                                            // -------------------------------------------
                                        }
                                    } else {
                                        answer_box(ui, &mut self.input, true);
                                    }


                                    if self.questions[idx].fails >= 1 {
                                        if let Some(hint) = &self.questions[idx].hint {
                                            ui.label(format!("💡 Pista: {hint}"));
                                        }
                                    }

                                    ui.add_space(5.0);

                                    // if ui.button("⚡ Marcar semana como completada (TEST)").clicked() {
                                    //     let week = self.current_week.unwrap_or(1);
                                    //     let language = self.selected_language.unwrap_or(Language::C);
                                    //     for q in self.questions.iter_mut() {
                                    //         if q.week == week && q.language == language {
                                    //             q.is_done = true;
                                    //             q.saw_solution = false;
                                    //             q.attempts = 1;
                                    //             q.fails = 0;
                                    //             q.skips = 0;
                                    //         }
                                    //     }
                                    //     self.save_progress();
                                    //     self.current_in_week = self.next_pending_in_week();
                                    //     // Si ya no quedan preguntas, muestra resumen
                                    //     if self.current_in_week.is_none() {
                                    //         self.state = AppState::Summary;
                                    //     }
                                    // }

                                    // Botones
                                    ui.horizontal(|ui| {
                                        ui.add_space((ui.available_width() - panel_width) / 2.0);
                                        let button_width = (panel_width - 8.0) / 2.0;
                                        let enviar = ui.add_sized([button_width, 36.0], egui::Button::new("Enviar"));
                                        let saltar = ui.add_sized([button_width, 36.0], egui::Button::new("Saltar pregunta"));

                                        if enviar.clicked() {
                                            if self.input.trim().is_empty() {
                                                self.message = "⚠ Debes escribir una respuesta antes de enviar.".to_string();
                                            } else {
                                                let user_code = normalize_code(&self.input);
                                                let answer_code = normalize_code(&self.questions[idx].answer);
                                                self.questions[idx].attempts += 1;

                                                // ¡Marca esta pregunta como mostrada en la ronda actual!
                                                if !self.shown_this_round.contains(&idx) {
                                                    self.shown_this_round.push(idx);
                                                }

                                                if user_code == answer_code {
                                                    self.message.clear();
                                                    self.questions[idx].is_done = true;
                                                    self.message = "✅ ¡Correcto!".to_string();
                                                    self.input.clear();
                                                    self.current_in_week = self.next_pending_in_week();

                                                    self.update_input_prefill();

                                                    if self.current_in_week.is_none() {

                                                        // Marca la semana como completada y desbloquea la siguiente
                                                        let week = self.current_week.unwrap_or(1);
                                                        self.complete_week(week);

                                                        self.state = AppState::Summary;
                                                    }
                                                } else {

                                                    self.questions[idx].fails += 1;
                                                    self.message = "❌ Incorrecto. Intenta de nuevo.".to_string();
                                                    self.input.clear();


                                                }
                                                self.save_progress();
                                            }

                                        }


                                        if saltar.clicked() {
                                            self.questions[idx].skips += 1;
                                            self.questions[idx].attempts += 1;
                                            self.message = "⏩ Pregunta saltada. La verás más adelante.".to_string();
                                            self.input.clear();

                                            if !self.shown_this_round.contains(&idx) {
                                                self.shown_this_round.push(idx);
                                            }

                                            self.current_in_week = self.next_pending_in_week();

                                            self.update_input_prefill();

                                            if self.current_in_week.is_none() {

                                                // Marca la semana como completada y desbloquea la siguiente
                                                let week = self.current_week.unwrap_or(1);
                                                self.complete_week(week);

                                                self.state = AppState::Summary;
                                            }
                                            self.save_progress();
                                        }


                                        self.save_progress();
                                    });

                                    ui.horizontal(|ui| {
                                        ui.add_space((ui.available_width() - panel_width) / 2.0);

                                        let button_width = (panel_width - 8.0) / 2.0;
                                        let guardar = ui.add_sized([button_width, 36.0], egui::Button::new("Guardar y salir"));
                                        let terminar = ui.add_sized([button_width, 36.0], egui::Button::new("Ver progreso"));

                                        if terminar.clicked() {
                                            self.state = AppState::Summary;
                                            self.message.clear();

                                        }

                                        if guardar.clicked() {
                                            self.save_progress();
                                            self.state = AppState::Welcome;
                                            self.message.clear();
                                        }
                                    });

                                    ui.add_space(8.0);

                                    if !self.message.is_empty() {
                                        ui.label(&self.message);
                                    }

                                }
                            });
                        });

                    ui.add_space(extra_space);
                });
            }

            // ----------- RESUMEN FINAL -----------
            AppState::Summary => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let max_width = 600.0;
                    let panel_width = (ui.available_width() * 0.97).min(max_width);
                    let button_width = panel_width / 3.0;
                    let button_height = 36.0;
                    let total_height = 700.0;
                    let extra_space = (ui.available_height() - total_height).max(0.0) / 2.0;

                    ui.add_space(extra_space);


                    ui.vertical_centered(|ui| {
                        egui::Frame::default()
                            .fill(ui.visuals().window_fill())
                            .inner_margin(egui::Margin::symmetric(16, 50))
                            .show(ui, |ui| {
                                ui.set_width(panel_width / 1.5);

                                ui.heading("Progreso Actual");
                                ui.add_space(10.0);
                                ui.label("Resumen de preguntas:");
                                ui.add_space(5.0);

                                let max_height = 700.0;

                                egui::ScrollArea::vertical()
                                    .max_height(max_height)
                                    .max_width(panel_width)
                                    .show(ui, |ui| {
                                        let week = self.current_week.unwrap_or(1);

                                        egui::Grid::new("quiz_results_grid")
                                            .striped(true)
                                            .spacing([8.0, 0.0])
                                            .show(ui, |ui| {
                                                ui.label("Pregunta");
                                                ui.label("Intentos");
                                                ui.label("Fallos");
                                                ui.label("Saltos");
                                                ui.label("Solución vista");
                                                ui.label("Estado");
                                                ui.end_row();

                                                for (i, q) in self.questions.iter().enumerate() {
                                                    if q.week != week { continue; }
                                                    let status = if q.is_done && !q.saw_solution {
                                                        "✅ Correcta"
                                                    } else if q.saw_solution {
                                                        "❌ Fallida"
                                                    } else {
                                                        "❌ Sin responder"
                                                    };
                                                    let solucion_vista = if q.saw_solution { "Sí" } else { "No" };
                                                    ui.label(format!("{}", i + 1));
                                                    ui.label(format!("{}", q.attempts));
                                                    ui.label(format!("{}", q.fails));
                                                    ui.label(format!("{}", q.skips));
                                                    ui.label(solucion_vista);
                                                    ui.label(status);
                                                    ui.end_row();
                                                }
                                            });
                                    });


                                ui.add_space(1.0);


                                // Aquí los botones, dentro del mismo bloque
                                ui.horizontal_centered(|ui| {
                                    let volver = ui.add_sized([button_width, button_height], egui::Button::new("Atrás"));

                                    let current_week = self.current_week.unwrap_or(1);
                                    let language = self.selected_language.unwrap_or(Language::C);
                                    let total_weeks = self.questions
                                        .iter()
                                        .filter(|q| q.language == language)
                                        .map(|q| q.week)
                                        .max()
                                        .unwrap_or(current_week);

                                    let is_current_week_complete = self.is_week_completed(current_week);
                                    let has_next_week = current_week < total_weeks;

                                    if volver.clicked() {
                                        if let Some(lang) = self.selected_language {
                                            *self = QuizApp::load_progress(lang)
                                                .unwrap_or_else(|| QuizApp::new_for_language(lang));
                                            self.state = AppState::Quiz;
                                        }
                                    }

                                    if is_current_week_complete && has_next_week {
                                        let siguiente = ui.add_sized([button_width, button_height], egui::Button::new("Siguiente Semana"));
                                        if siguiente.clicked() {
                                            let next_week = current_week + 1;
                                            self.select_week(next_week);
                                            self.recalculate_unlocked_weeks();
                                            self.update_input_prefill();
                                            self.save_progress();
                                            self.state = AppState::Quiz;
                                            self.message.clear();
                                        }
                                    } else {
                                        let terminar = ui.add_sized([button_width, button_height], egui::Button::new("Terminar"));
                                        if terminar.clicked() {
                                            self.state = AppState::Welcome;
                                        }
                                    }
                                });
                            });
                    });



                });
            }

        }
    }

}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "SummerQuiz - Telegram: @sugarRayL",
        options,
        Box::new(|cc| {
            // Detectar tema preferido del sistema
            let prefers_dark = cc.egui_ctx.style().visuals.dark_mode;
            if prefers_dark {
                cc.egui_ctx.set_visuals(Visuals::dark());
            } else {
                cc.egui_ctx.set_visuals(Visuals::light());
            }

            let quiz = QuizApp::new();
            Ok(Box::new(quiz))
        }),
    )
}



