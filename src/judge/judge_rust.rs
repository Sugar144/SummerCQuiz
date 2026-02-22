#[cfg(target_arch = "wasm32")]
use crate::judge::judge_c::JudgeResult;
#[cfg(not(target_arch = "wasm32"))]
mod native_rust {
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

    #[derive(Clone)]
    enum SubmissionSource {
        Raw,
        WrappedBody,
    }

    pub fn grade_rust_question(question: &Question, user_code: &str) -> JudgeResult {
        if question.tests.is_empty() {
            return JudgeResult::InfrastructureError {
                message: "La pregunta judge_rust no tiene tests configurados.".into(),
            };
        }

        let rustc = match detect_rustc() {
            Ok(path) => path,
            Err(msg) => return JudgeResult::InfrastructureError { message: msg },
        };

        let cache_dir = match cache_dir() {
            Ok(dir) => dir,
            Err(err) => {
                return JudgeResult::InfrastructureError {
                    message: format!("No se pudo preparar el cache del juez Rust: {err}"),
                };
            }
        };

        let candidates = build_source_candidates(user_code);
        let mut first_compile_error = None;

        for (source, source_kind) in candidates {
            let binary_path = build_cached_binary(&cache_dir, &rustc, &source, &source_kind);
            if !binary_path.exists() {
                if let Err(stderr) = compile_source(&rustc, &source, &binary_path) {
                    if first_compile_error.is_none() {
                        first_compile_error = Some(stderr);
                    }
                    continue;
                }
            }

            return run_tests(&binary_path, &question.tests);
        }

        JudgeResult::CompileError {
            stderr: first_compile_error
                .unwrap_or_else(|| "Error de compilación desconocido en Rust.".into()),
        }
    }

    fn detect_rustc() -> Result<PathBuf, String> {
        if let Ok(status) = Command::new("rustc")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            if status.success() {
                return Ok(PathBuf::from("rustc"));
            }
        }

        Err("No se encontró 'rustc' en PATH.".into())
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

        let dir = base.join("summer_quiz").join("judge_rust_cache");
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn build_source_candidates(user_code: &str) -> Vec<(String, SubmissionSource)> {
        let mut candidates = Vec::new();
        candidates.push((user_code.to_string(), SubmissionSource::Raw));

        if !contains_main(user_code) {
            candidates.push((wrap_as_main_body(user_code), SubmissionSource::WrappedBody));
        }

        candidates
    }

    fn contains_main(code: &str) -> bool {
        let normalized = code.replace(char::is_whitespace, "");
        normalized.contains("fnmain(")
    }

    fn wrap_as_main_body(code: &str) -> String {
        format!(
            "fn main() {{
                {code}
            }}"
        )
    }

    fn build_cached_binary(
        cache_dir: &Path,
        rustc: &Path,
        source: &str,
        source_kind: &SubmissionSource,
    ) -> PathBuf {
        let mut hasher = DefaultHasher::new();
        rustc.to_string_lossy().hash(&mut hasher);
        "-O".hash(&mut hasher);
        source.hash(&mut hasher);
        std::mem::discriminant(source_kind).hash(&mut hasher);
        let key = format!("{:016x}", hasher.finish());
        if cfg!(target_os = "windows") {
            cache_dir.join(format!("{key}.exe"))
        } else {
            cache_dir.join(key)
        }
    }

    fn compile_source(rustc: &Path, source: &str, out_bin: &Path) -> Result<(), String> {
        let src_path = out_bin.with_extension("rs");
        fs::write(&src_path, source)
            .map_err(|err| format!("No se pudo guardar Rust temporal: {err}"))?;

        let output = Command::new(rustc)
            .arg(&src_path)
            .arg("-O")
            .arg("-o")
            .arg(out_bin)
            .output()
            .map_err(|err| format!("No se pudo invocar rustc: {err}"))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    fn run_tests(binary_path: &Path, tests: &[JudgeTestCase]) -> JudgeResult {
        for (idx, test) in tests.iter().enumerate() {
            let exec = execute_test(binary_path, test, idx + 1, TIMEOUT_MS);
            match exec {
                JudgeResult::Accepted => continue,
                other => return other,
            }
        }
        JudgeResult::Accepted
    }

    fn execute_test(
        binary_path: &Path,
        test: &JudgeTestCase,
        test_index: usize,
        timeout_ms: u64,
    ) -> JudgeResult {
        let mut child = match Command::new(binary_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(err) => {
                return JudgeResult::InfrastructureError {
                    message: format!("No se pudo ejecutar el binario Rust compilado: {err}"),
                };
            }
        };

        if let Some(mut stdin) = child.stdin.take() {
            if let Err(err) = stdin.write_all(test.input.as_bytes()) {
                return JudgeResult::InfrastructureError {
                    message: format!("No se pudo enviar stdin al programa Rust: {err}"),
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
                                    "No se pudo leer la salida del programa Rust: {err}"
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
                        message: format!("Error esperando al programa Rust: {err}"),
                    };
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native_rust::grade_rust_question;

#[cfg(target_arch = "wasm32")]
pub fn grade_rust_question(
    _question: &crate::model::Question,
    _user_code: &str,
) -> JudgeResult {
     JudgeResult::InfrastructureError {
        message: "El juez Rust no está disponible en WASM.".into(),
    }
}
