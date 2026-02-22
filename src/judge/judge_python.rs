#[cfg(target_arch = "wasm32")]
use crate::judge::judge_c::JudgeResult;
#[cfg(not(target_arch = "wasm32"))]
mod native_python {
    use crate::judge::judge_c::JudgeResult;
    use crate::judge::judge_utils::{line_diff, matches_expected_output, normalize_newlines};
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

    pub fn grade_python_question(question: &Question, user_code: &str) -> JudgeResult {
        if question.tests.is_empty() {
            return JudgeResult::InfrastructureError {
                message: "La pregunta judge_python no tiene tests configurados.".into(),
            };
        }

        let python = match detect_python() {
            Ok(path) => path,
            Err(msg) => return JudgeResult::InfrastructureError { message: msg },
        };

        let cache_dir = match cache_dir() {
            Ok(dir) => dir,
            Err(err) => {
                return JudgeResult::InfrastructureError {
                    message: format!("No se pudo preparar el cache del juez Python: {err}"),
                };
            }
        };

        let script_path = build_cached_script_path(&cache_dir, &python, user_code);
        if !script_path.exists() {
            if let Err(err) = fs::write(&script_path, user_code) {
                return JudgeResult::InfrastructureError {
                    message: format!("No se pudo guardar el script Python temporal: {err}"),
                };
            }

            if let Err(stderr) = compile_python(&python, &script_path) {
                return JudgeResult::CompileError { stderr };
            }
        }

        run_tests(&python, &script_path, &question.tests)
    }

    fn detect_python() -> Result<PathBuf, String> {
        for candidate in ["python3", "python"] {
            if let Ok(status) = Command::new(candidate)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
            {
                if status.success() {
                    return Ok(PathBuf::from(candidate));
                }
            }
        }

        Err("No se encontró 'python3' ni 'python' en PATH.".into())
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

        let dir = base.join("summer_quiz").join("judge_python_cache");
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn build_cached_script_path(cache_dir: &Path, python: &Path, source: &str) -> PathBuf {
        let mut hasher = DefaultHasher::new();
        python.to_string_lossy().hash(&mut hasher);
        source.hash(&mut hasher);
        let key = format!("{:016x}", hasher.finish());
        cache_dir.join(format!("{key}.py"))
    }

    fn compile_python(python: &Path, script_path: &Path) -> Result<(), String> {
        let output = Command::new(python)
            .arg("-m")
            .arg("py_compile")
            .arg(script_path)
            .output()
            .map_err(|err| format!("No se pudo invocar Python para compilar: {err}"))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    fn run_tests(python: &Path, script_path: &Path, tests: &[JudgeTestCase]) -> JudgeResult {
        for (idx, test) in tests.iter().enumerate() {
            let exec = execute_test(python, script_path, test, idx + 1, TIMEOUT_MS);
            match exec {
                JudgeResult::Accepted => continue,
                other => return other,
            }
        }
        JudgeResult::Accepted
    }

    fn execute_test(
        python: &Path,
        script_path: &Path,
        test: &JudgeTestCase,
        test_index: usize,
        timeout_ms: u64,
    ) -> JudgeResult {
        let mut child = match Command::new(python)
            .arg(script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(err) => {
                return JudgeResult::InfrastructureError {
                    message: format!("No se pudo ejecutar el script Python: {err}"),
                };
            }
        };

        if let Some(mut stdin) = child.stdin.take() {
            if let Err(err) = stdin.write_all(test.input.as_bytes()) {
                return JudgeResult::InfrastructureError {
                    message: format!("No se pudo enviar stdin al script Python: {err}"),
                };
            }
        }

        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    let output = match child.wait_with_output() {
                        Ok(output) => output,
                        Err(err) => {
                            return JudgeResult::InfrastructureError {
                                message: format!(
                                    "No se pudo leer la salida del script Python: {err}"
                                ),
                            };
                        }
                    };

                    let received = normalize_newlines(&String::from_utf8_lossy(&output.stdout));
                    let expected = normalize_newlines(&test.output);

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
                        message: format!("Error esperando al script Python: {err}"),
                    };
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native_python::grade_python_question;

#[cfg(target_arch = "wasm32")]
pub fn grade_python_question(
    _question: &crate::model::Question,
    _user_code: &str,
) -> JudgeResult {
     JudgeResult::InfrastructureError {
        message: "El juez Python no está disponible en WASM.".into(),
    }
}
