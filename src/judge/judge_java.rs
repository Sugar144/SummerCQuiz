#[cfg(not(target_arch = "wasm32"))]
mod native_java {
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

    pub fn grade_java_question(question: &Question, user_code: &str) -> JudgeResult {
        if question.tests.is_empty() {
            return JudgeResult::InfrastructureError {
                message: "La pregunta judge_java no tiene tests configurados.".into(),
            };
        }

        let javac = match detect_javac() {
            Ok(path) => path,
            Err(msg) => return JudgeResult::InfrastructureError { message: msg },
        };

        if let Err(msg) = detect_java_runtime() {
            return JudgeResult::InfrastructureError { message: msg };
        }

        let cache_dir = match cache_dir() {
            Ok(dir) => dir,
            Err(err) => {
                return JudgeResult::InfrastructureError {
                    message: format!("No se pudo preparar el cache del juez Java: {err}"),
                };
            }
        };

        let candidates = build_source_candidates(user_code);
        let mut first_compile_error = None;

        for (source, source_kind) in candidates {
            let class_dir = build_cached_class_dir(&cache_dir, &javac, &source, &source_kind);
            if !class_dir.join("Main.class").exists() {
                if let Err(stderr) = compile_source(&javac, &source, &class_dir) {
                    if first_compile_error.is_none() {
                        first_compile_error = Some(stderr);
                    }
                    continue;
                }
            }

            return run_tests(&class_dir, &question.tests);
        }

        JudgeResult::CompileError {
            stderr: first_compile_error
                .unwrap_or_else(|| "Error de compilación desconocido en Java.".into()),
        }
    }

    fn detect_javac() -> Result<PathBuf, String> {
        if let Ok(status) = Command::new("javac")
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            if status.success() {
                return Ok(PathBuf::from("javac"));
            }
        }

        Err("No se encontró 'javac' en PATH.".into())
    }

    fn detect_java_runtime() -> Result<(), String> {
        if let Ok(status) = Command::new("java")
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            if status.success() {
                return Ok(());
            }
        }

        Err("No se encontró 'java' en PATH.".into())
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

        let dir = base.join("summer_quiz").join("judge_java_cache");
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn build_source_candidates(user_code: &str) -> Vec<(String, SubmissionSource)> {
        let mut candidates = Vec::new();
        candidates.push((user_code.to_string(), SubmissionSource::Raw));

        if !contains_main_class(user_code) {
            candidates.push((wrap_as_main_body(user_code), SubmissionSource::WrappedBody));
        }

        candidates
    }

    fn contains_main_class(code: &str) -> bool {
        let normalized = code.replace(char::is_whitespace, "").to_lowercase();
        normalized.contains("classmain")
            && normalized.contains("publicstaticvoidmain(string[]args)")
    }

    fn wrap_as_main_body(code: &str) -> String {
        format!(
            "import java.util.*;
            class Main {{
                public static void main(String[] args) {{
                    {code}
                }}
            }}"
        )
    }

    fn build_cached_class_dir(
        cache_dir: &Path,
        javac: &Path,
        source: &str,
        source_kind: &SubmissionSource,
    ) -> PathBuf {
        let mut hasher = DefaultHasher::new();
        javac.to_string_lossy().hash(&mut hasher);
        source.hash(&mut hasher);
        std::mem::discriminant(source_kind).hash(&mut hasher);
        let key = format!("{:016x}", hasher.finish());
        cache_dir.join(key)
    }

    fn compile_source(javac: &Path, source: &str, out_dir: &Path) -> Result<(), String> {
        fs::create_dir_all(out_dir)
            .map_err(|err| format!("No se pudo crear directorio temporal Java: {err}"))?;

        let src_path = out_dir.join("Main.java");
        fs::write(&src_path, source)
            .map_err(|err| format!("No se pudo guardar Java temporal: {err}"))?;

        let output = Command::new(javac)
            .arg(&src_path)
            .arg("-d")
            .arg(out_dir)
            .output()
            .map_err(|err| format!("No se pudo invocar javac: {err}"))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    fn run_tests(class_dir: &Path, tests: &[JudgeTestCase]) -> JudgeResult {
        for (idx, test) in tests.iter().enumerate() {
            let exec = execute_test(class_dir, test, idx + 1, TIMEOUT_MS);
            match exec {
                JudgeResult::Accepted => continue,
                other => return other,
            }
        }
        JudgeResult::Accepted
    }

    fn execute_test(
        class_dir: &Path,
        test: &JudgeTestCase,
        test_index: usize,
        timeout_ms: u64,
    ) -> JudgeResult {
        let mut child = match Command::new("java")
            .arg("-cp")
            .arg(class_dir)
            .arg("Main")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(err) => {
                return JudgeResult::InfrastructureError {
                    message: format!("No se pudo ejecutar la clase Java compilada: {err}"),
                };
            }
        };

        if let Some(mut stdin) = child.stdin.take() {
            if let Err(err) = stdin.write_all(test.input.as_bytes()) {
                return JudgeResult::InfrastructureError {
                    message: format!("No se pudo enviar stdin al programa Java: {err}"),
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
                                    "No se pudo leer la salida del programa Java: {err}"
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
                        message: format!("Error esperando al programa Java: {err}"),
                    };
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native_java::grade_java_question;

#[cfg(target_arch = "wasm32")]
pub fn grade_java_question(
    _question: &crate::model::Question,
    _user_code: &str,
) -> crate::judge_c::JudgeResult {
    crate::judge_c::JudgeResult::InfrastructureError {
        message: "El juez Java no está disponible en WASM.".into(),
    }
}
