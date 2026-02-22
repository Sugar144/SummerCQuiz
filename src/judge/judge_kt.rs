#[cfg(target_arch = "wasm32")]
use crate::judge::judge_c::JudgeResult;
#[cfg(not(target_arch = "wasm32"))]
mod native_kotlin {
    use crate::judge::judge_c::JudgeResult;
    use crate::judge::judge_utils::{line_diff, matches_expected_output};
    use crate::model::{JudgeTestCase, Question};
    use std::collections::hash_map::DefaultHasher;
    use std::env;
    use std::fs;
    use std::hash::{Hash, Hasher};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

    const TIMEOUT_MS: u64 = 2_000;
    const POLL_MS: u64 = 10;

    pub fn grade_kotlin_question(question: &Question, user_code: &str) -> JudgeResult {
        if question.tests.is_empty() {
            return JudgeResult::InfrastructureError {
                message: "La pregunta judge_kotlin no tiene tests configurados.".into(),
            };
        }

        let kotlinc = match detect_kotlinc() {
            Ok(p) => p,
            Err(msg) => return JudgeResult::InfrastructureError { message: msg },
        };

        let cache_dir = match cache_dir() {
            Ok(dir) => dir,
            Err(err) => {
                return JudgeResult::InfrastructureError {
                    message: format!("No se pudo preparar el cache del juez Kotlin: {err}"),
                };
            }
        };

        let jar_path = build_cached_jar(&cache_dir, &kotlinc, user_code);
        if !jar_path.exists() {
            if let Err(stderr) = compile_kotlin(&kotlinc, user_code, &jar_path) {
                return JudgeResult::CompileError { stderr };
            }
        }

        run_tests_jar(&jar_path, &question.tests)
    }

    fn detect_kotlinc() -> Result<PathBuf, String> {
        if let Ok(status) = Command::new("kotlinc")
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            if status.success() {
                return Ok(PathBuf::from("kotlinc"));
            }
        }

        Err("No se encontró 'kotlinc' en PATH.\n\
             Linux: instala Kotlin (ej. sdkman) y asegúrate de tener kotlinc.\n\
             Windows: instala Kotlin compiler o usa SDKMAN en WSL.\n\
             También necesitas 'java' (JRE/JDK) para ejecutar el .jar."
            .into())
    }

    fn cache_dir() -> Result<PathBuf, std::io::Error> {
        let base = if cfg!(target_os = "windows") {
            env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .or_else(|| env::var_os("TEMP").map(PathBuf::from))
                .unwrap_or_else(env::temp_dir)
        } else {
            env::var_os("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
                .unwrap_or_else(env::temp_dir)
        };

        let dir = base.join("summer_quiz").join("judge_kotlin_cache");
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn build_cached_jar(cache_dir: &Path, kotlinc: &Path, source: &str) -> PathBuf {
        let mut hasher = DefaultHasher::new();
        kotlinc.to_string_lossy().hash(&mut hasher);
        "-include-runtime".hash(&mut hasher);
        source.hash(&mut hasher);
        let key = format!("{:016x}", hasher.finish());
        cache_dir.join(format!("{key}.jar"))
    }

    fn compile_kotlin(kotlinc: &Path, source: &str, out_jar: &Path) -> Result<(), String> {
        // Asegura que exista main si tu UX lo requiere.
        // (Si quieres: detectas "fun main" y si no lo hay, envuelves.)
        let src_path = out_jar.with_extension("kt");
        fs::write(&src_path, source)
            .map_err(|e| format!("No se pudo guardar Kotlin temporal: {e}"))?;

        let output = Command::new(kotlinc)
            .arg(&src_path)
            .arg("-include-runtime")
            .arg("-d")
            .arg(out_jar)
            .output()
            .map_err(|e| format!("No se pudo invocar kotlinc: {e}"))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    fn run_tests_jar(jar_path: &Path, tests: &[JudgeTestCase]) -> JudgeResult {
        for (idx, test) in tests.iter().enumerate() {
            let exec = execute_test_jar(jar_path, test, idx + 1, TIMEOUT_MS);
            match exec {
                JudgeResult::Accepted => continue,
                other => return other,
            }
        }
        JudgeResult::Accepted
    }

    fn execute_test_jar(
        jar_path: &Path,
        test: &JudgeTestCase,
        test_index: usize,
        timeout_ms: u64,
    ) -> JudgeResult {
        let mut child = match Command::new("java")
            .arg("-jar")
            .arg(jar_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(err) => {
                return JudgeResult::InfrastructureError {
                    message: format!("No se pudo ejecutar java -jar: {err}"),
                };
            }
        };

        if let Some(mut stdin) = child.stdin.take() {
            if let Err(err) = stdin.write_all(test.input.as_bytes()) {
                return JudgeResult::InfrastructureError {
                    message: format!("No se pudo enviar stdin: {err}"),
                };
            }
        }

        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    let output = match child.wait_with_output() {
                        Ok(o) => o,
                        Err(err) => {
                            return JudgeResult::InfrastructureError {
                                message: format!("No se pudo leer la salida: {err}"),
                            };
                        }
                    };

                    // Reutiliza tus helpers: normalize_newlines, matches_expected_output, line_diff…
                    let received = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
                    let expected = test.output.replace("\r\n", "\n");

                    if output.status.code().unwrap_or(-1) != 0 {
                        return JudgeResult::RuntimeError {
                            test_index,
                            input: test.input.clone(),
                            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                            exit_code: output.status.code(),
                        };
                    }

                    if !matches_expected_output(&received, &expected) {
                        return JudgeResult::WrongAnswer {
                            test_index,
                            input: test.input.clone(),
                            expected,
                            received: received.clone(),
                            diff: line_diff(&test.output, &received),
                        };
                    }

                    return JudgeResult::Accepted;
                }
                Ok(None) => {
                    if start.elapsed() > Duration::from_millis(timeout_ms) {
                        let _ = child.kill();
                        let _ = child.wait();
                        return JudgeResult::Timeout {
                            test_index,
                            input: test.input.clone(),
                            timeout_ms,
                        };
                    }
                    thread::sleep(Duration::from_millis(POLL_MS));
                }
                Err(err) => {
                    return JudgeResult::InfrastructureError {
                        message: format!("Error esperando al programa: {err}"),
                    };
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native_kotlin::grade_kotlin_question;

#[cfg(target_arch = "wasm32")]
pub fn grade_kotlin_question(_question: &crate::model::Question, _user_code: &str) -> JudgeResult {
    JudgeResult::InfrastructureError {
        message: "El juez Kotlin no está disponible en WASM.".into(),
    }
}
