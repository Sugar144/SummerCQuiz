use crate::judge::judge_c::JudgeResult;
use crate::model::Question;

#[cfg(not(target_arch = "wasm32"))]
mod native_remote {
    use super::*;
    use reqwest::blocking::Client;
    use reqwest::header::CONTENT_TYPE;
    use serde::{Deserialize, Serialize};

    const DEFAULT_REMOTE_JUDGE_URL: &str = "http://127.0.0.1:8787/api/judge/sync";

    #[derive(Debug, Serialize)]
    struct RemoteJudgeRequest<'a> {
        language: &'a str,
        source: &'a str,
        tests: &'a [crate::model::JudgeTestCase],
        harness: Option<&'a str>,
        question_id: Option<&'a str>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(tag = "status", rename_all = "snake_case")]
    enum RemoteJudgeResponse {
        Accepted,
        CompileError {
            stderr: String,
        },
        WrongAnswer {
            test_index: usize,
            input: String,
            expected: String,
            received: String,
            #[serde(default)]
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

    pub fn grade_remote_question(question: &Question, user_code: &str) -> JudgeResult {
        if question.tests.is_empty() {
            return JudgeResult::InfrastructureError {
                message: "La pregunta judge_remote no tiene tests configurados.".into(),
            };
        }

        let endpoint = question
            .judge_endpoint
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(DEFAULT_REMOTE_JUDGE_URL);

        let payload = RemoteJudgeRequest {
            language: map_language(question),
            source: user_code,
            tests: &question.tests,
            harness: question.judge_harness.as_deref(),
            question_id: question.id.as_deref(),
        };

        let client = Client::new();
        let response = match client
            .post(endpoint)
            .header(CONTENT_TYPE, "application/json")
            .json(&payload)
            .send()
        {
            Ok(response) => response,
            Err(err) => {
                return JudgeResult::InfrastructureError {
                    message: format!("No se pudo contactar el judge remoto ({endpoint}): {err}"),
                };
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return JudgeResult::InfrastructureError {
                message: format!("Judge remoto devolvió HTTP {status}. Body: {body}"),
            };
        }

        let parsed = match response.json::<RemoteJudgeResponse>() {
            Ok(parsed) => parsed,
            Err(err) => {
                return JudgeResult::InfrastructureError {
                    message: format!("Respuesta inválida del judge remoto: {err}"),
                };
            }
        };

        map_response(parsed)
    }

    fn map_language(question: &Question) -> &'static str {
        match question.language {
            crate::model::Language::C => "c",
            crate::model::Language::Pseudocode => "pseudocode",
            crate::model::Language::Kotlin => "kotlin",
            crate::model::Language::Java => "java",
            crate::model::Language::Rust => "rust",
            crate::model::Language::Python => "python",
            crate::model::Language::GitGithub => "git_github",
        }
    }

    fn map_response(value: RemoteJudgeResponse) -> JudgeResult {
        match value {
            RemoteJudgeResponse::Accepted => JudgeResult::Accepted,
            RemoteJudgeResponse::CompileError { stderr } => JudgeResult::CompileError { stderr },
            RemoteJudgeResponse::WrongAnswer {
                test_index,
                input,
                expected,
                received,
                diff,
            } => JudgeResult::WrongAnswer {
                test_index,
                input,
                expected,
                received,
                diff,
            },
            RemoteJudgeResponse::Timeout {
                test_index,
                input,
                timeout_ms,
            } => JudgeResult::Timeout {
                test_index,
                input,
                timeout_ms,
            },
            RemoteJudgeResponse::RuntimeError {
                test_index,
                input,
                stderr,
                exit_code,
            } => JudgeResult::RuntimeError {
                test_index,
                input,
                stderr,
                exit_code,
            },
            RemoteJudgeResponse::InfrastructureError { message } => {
                JudgeResult::InfrastructureError { message }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native_remote::grade_remote_question;

#[cfg(target_arch = "wasm32")]
pub fn grade_remote_question(_question: &Question, _user_code: &str) -> JudgeResult {
    JudgeResult::InfrastructureError {
        message: "judge_remote en WASM requiere flujo asíncrono (submit/poll) aún no implementado.".into(),
    }
}
