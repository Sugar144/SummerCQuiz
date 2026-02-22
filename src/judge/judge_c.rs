use crate::model::Question;

#[derive(Debug, Clone)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub enum JudgeResult {
    Accepted,
    CompileError {
        stderr: String,
    },
    WrongAnswer {
        test_index: usize,
        input: String,
        expected: String,
        received: String,
        diff: String,
    },
    Timeout {
        test_index: usize,
        input: String,
        timeout_ms: u64,
    },
    RuntimeError {
        test_index: usize,
        input: String,
        stderr: String,
        exit_code: Option<i32>,
    },
    InfrastructureError {
        message: String,
    },
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::JudgeResult;
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
        CustomHarness,
    }

    pub fn grade_c_question(question: &Question, user_code: &str) -> JudgeResult {
        if question.tests.is_empty() {
            return JudgeResult::InfrastructureError {
                message: "La pregunta judge_c no tiene tests configurados.".into(),
            };
        }

        let compiler = match detect_compiler() {
            Ok(path) => path,
            Err(msg) => return JudgeResult::InfrastructureError { message: msg },
        };

        let cache_dir = match cache_dir() {
            Ok(dir) => dir,
            Err(err) => {
                return JudgeResult::InfrastructureError {
                    message: format!("No se pudo preparar el cache del juez: {err}"),
                };
            }
        };

        let candidates = build_source_candidates(question, user_code);
        let mut first_compile_error = None;

        for (source, source_kind) in candidates {
            let binary_path = build_cached_binary(&cache_dir, &compiler, &source, &source_kind);
            if !binary_path.exists() {
                if let Err(stderr) = compile_source(&compiler, &source, &binary_path) {
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
                .unwrap_or_else(|| "Error de compilación desconocido.".into()),
        }
    }

    fn build_source_candidates(
        question: &Question,
        user_code: &str,
    ) -> Vec<(String, SubmissionSource)> {
        let mut candidates = Vec::new();

        if let Some(harness) = question.judge_harness.as_deref() {
            candidates.push((
                apply_harness(user_code, Some(harness)),
                SubmissionSource::CustomHarness,
            ));
            return candidates;
        }

        candidates.push((user_code.to_string(), SubmissionSource::Raw));

        if !contains_main(user_code) {
            candidates.push((wrap_as_main_body(user_code), SubmissionSource::WrappedBody));
        }

        candidates
    }

    fn contains_main(code: &str) -> bool {
        let normalized = code.replace(char::is_whitespace, "");
        normalized.contains("main(")
    }

    fn wrap_as_main_body(code: &str) -> String {
        format!(
            "#include <stdio.h>
            #include <stdbool.h>

            int main(void) {{
                {code}
                return 0;
            }}
            "
        )
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
                    message: format!("No se pudo ejecutar el binario compilado: {err}"),
                };
            }
        };

        if let Some(mut stdin) = child.stdin.take() {
            if let Err(err) = stdin.write_all(test.input.as_bytes()) {
                return JudgeResult::InfrastructureError {
                    message: format!("No se pudo enviar stdin al programa: {err}"),
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
                                message: format!("No se pudo leer la salida del programa: {err}"),
                            };
                        }
                    };
                    return evaluate_output(
                        test,
                        test_index,
                        output.status.code(),
                        &output.stdout,
                        &output.stderr,
                    );
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

    fn evaluate_output(
        test: &JudgeTestCase,
        test_index: usize,
        exit_code: Option<i32>,
        stdout: &[u8],
        stderr: &[u8],
    ) -> JudgeResult {
        let received = normalize_newlines(&String::from_utf8_lossy(stdout));
        let expected = normalize_newlines(&test.output);

        if exit_code.unwrap_or(-1) != 0 {
            return JudgeResult::RuntimeError {
                test_index,
                input: test.input.clone(),
                stderr: String::from_utf8_lossy(stderr).to_string(),
                exit_code,
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

        JudgeResult::Accepted
    }

    fn apply_harness(user_code: &str, harness: Option<&str>) -> String {
        if let Some(harness) = harness {
            if harness.contains("{{USER_CODE}}") {
                return harness.replace("{{USER_CODE}}", user_code);
            }
            return format!("{user_code}\n{harness}\n");
        }
        user_code.to_string()
    }

    fn compile_source(compiler: &Path, source: &str, output_binary: &Path) -> Result<(), String> {
        let source_path = output_binary.with_extension("c");
        fs::write(&source_path, source)
            .map_err(|e| format!("No se pudo guardar código temporal: {e}"))?;

        let output = Command::new(compiler)
            .arg(&source_path)
            .arg("-std=c11")
            .arg("-O2")
            .arg("-o")
            .arg(output_binary)
            .output()
            .map_err(|e| format!("No se pudo invocar al compilador: {e}"))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    fn detect_compiler() -> Result<PathBuf, String> {
        for candidate in ["clang", "gcc"] {
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

        Err("No se encontró un compilador C (clang/gcc) en PATH.
Linux: instala clang o gcc (ej. sudo apt install clang).
Windows: instala MSYS2/MinGW y agrega gcc/clang a PATH."
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

        let dir = base.join("summer_quiz").join("judge_c_cache");
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn build_cached_binary(
        cache_dir: &Path,
        compiler: &Path,
        source: &str,
        source_kind: &SubmissionSource,
    ) -> PathBuf {
        let mut hasher = DefaultHasher::new();
        compiler.to_string_lossy().hash(&mut hasher);
        "-std=c11 -O2".hash(&mut hasher);
        source.hash(&mut hasher);
        std::mem::discriminant(source_kind).hash(&mut hasher);
        let key = format!("{:016x}", hasher.finish());
        let ext = if cfg!(target_os = "windows") {
            "exe"
        } else {
            "bin"
        };
        cache_dir.join(format!("{key}.{ext}"))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn grade_c_question(question: &Question, user_code: &str) -> JudgeResult {
    native::grade_c_question(question, user_code)
}

#[cfg(target_arch = "wasm32")]
pub fn grade_c_question(_question: &Question, _user_code: &str) -> JudgeResult {
    JudgeResult::InfrastructureError {
        message: "El juez C no está disponible en la versión web (wasm32).".into(),
    }
}

pub fn format_judge_message(result: &JudgeResult) -> String {
    match result {
        JudgeResult::Accepted => "✅ ¡Correcto!".into(),
        JudgeResult::CompileError { stderr } => {
            format!("❌ Error de compilación.\n\n{}", stderr.trim())
        }
        JudgeResult::WrongAnswer {
            test_index,
            input,
            expected,
            received,
            diff,
        } => format!(
            "❌ Wrong Answer (caso #{test_index}).\n\nInput:\n{input}\n\nEsperado:\n{expected}\n\nRecibido:\n{received}\n\nDiff:\n{diff}"
        ),
        JudgeResult::Timeout {
            test_index,
            input,
            timeout_ms,
        } => format!("❌ Timeout en caso #{test_index} ({timeout_ms} ms).\n\nInput:\n{input}"),
        JudgeResult::RuntimeError {
            test_index,
            input,
            stderr,
            exit_code,
        } => format!(
            "❌ Runtime Error en caso #{test_index} (exit code: {}).\n\nInput:\n{input}\n\nStderr:\n{}",
            exit_code
                .map(|v| v.to_string())
                .unwrap_or_else(|| "desconocido".into()),
            stderr.trim()
        ),
        JudgeResult::InfrastructureError { message } => format!("⚠ {message}"),
    }
}

pub fn should_use_judge(question: &Question) -> bool {
    question.uses_judge_c()
}
