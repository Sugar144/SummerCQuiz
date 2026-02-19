use super::*;
use crate::update::descargar_binario_nuevo;

impl QuizApp {
    pub fn ensure_update_thread(&mut self) {
        if self.update_thread_launched {
            return;
        }
        self.update_thread_launched = true;

        // El nombre del updater según plataforma
        let updater = if cfg!(windows) {
            "summer_quiz_updater.exe".to_string()
        } else {
            "./summer_quiz_updater".to_string()
        };

        // Hilo que descarga y arranca el updater
        std::thread::spawn(move || {
            match descargar_binario_nuevo() {
                Ok(()) => {
                    // Pequeña pausa para que el mensaje se vea
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    // Lanza el updater y sale
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
}
