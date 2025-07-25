use std::{thread, time::Duration, fs, env};

fn main() {
    thread::sleep(Duration::from_secs(2));

    let (old_name, new_name) = if cfg!(windows) {
        ("summer_quiz_bin.exe", "summer_quiz_bin_new.exe")
    } else {
        ("summer_quiz_bin", "summer_quiz_bin_new")
    };

    let exe_path = env::current_exe().expect("No se pudo obtener la ruta del ejecutable");
    let current_dir = exe_path.parent().expect("No tiene directorio padre");

    let old = current_dir.join(old_name);
    let new = current_dir.join(new_name);

    if new.exists() {
        fs::remove_file(&old).ok();
        fs::rename(&new, &old).expect("No se pudo actualizar el binario");
    }

    thread::sleep(Duration::from_secs(1));

    let mut waited = 0;
    while !old.exists() && waited < 5 {
        thread::sleep(Duration::from_secs(1));
        waited += 1;
    }

    let signal_file = current_dir.join(".update_success");
    fs::write(&signal_file, b"ok").ok();

    let launch_path = old.to_str().expect("Ruta no válida");
    let mut cmd = std::process::Command::new(launch_path);
    cmd.current_dir(&current_dir);

    match cmd.spawn() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error al relanzar la app: {e}");
        }
    }
}
