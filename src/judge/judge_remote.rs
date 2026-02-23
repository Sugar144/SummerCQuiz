use crate::judge::judge_c::JudgeResult;
use crate::model::{JudgeTestCase, Language, Question};
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
const DEFAULT_ENDPOINT: &str = "/api/judge/sync";
#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_NATIVE_ENDPOINT: &str = "http://127.0.0.1:8787/api/judge/sync";

#[derive(Debug, Serialize)]
struct JudgeRequest {
    language: String,
    source: String,
    tests: Vec<JudgeTestCase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    harness: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    question_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum JudgeResponse {
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

fn endpoint_for(question: &Question) -> String {
    question
        .judge_endpoint
        .clone()
        .unwrap_or_else(default_endpoint)
}

fn endpoint_candidates(primary: &str) -> Vec<String> {
    fn push_unique(candidates: &mut Vec<String>, value: String) {
        if !value.trim().is_empty() && !candidates.iter().any(|c| c == &value) {
            candidates.push(value);
        }
    }

    fn trim_trailing_slashes(value: &str) -> String {
        let trimmed = value.trim();
        if trimmed == "/" {
            return trimmed.to_string();
        }

        trimmed.trim_end_matches('/').to_string()
    }

    fn split_origin(value: &str) -> Option<(&str, &str)> {
        let scheme = value.find("://")?;
        let path_start = value[scheme + 3..].find('/').map(|i| i + scheme + 3);
        match path_start {
            Some(i) => Some((&value[..i], &value[i..])),
            None => Some((value, "")),
        }
    }

    fn push_suffixes(candidates: &mut Vec<String>, base: &str) {
        let base = if base == "/" {
            String::new()
        } else {
            base.to_string()
        };

        for suffix in ["/api/judge/sync", "/api/judge", "/judge/sync", "/judge"] {
            push_unique(candidates, format!("{base}{suffix}"));
        }
    }

    let mut candidates = Vec::new();
    let primary = trim_trailing_slashes(primary);
    push_unique(&mut candidates, primary.clone());

    if primary.starts_with("http://") || primary.starts_with("https://") {
        if let Some((origin, path)) = split_origin(&primary) {
            if path.is_empty() {
                push_suffixes(&mut candidates, origin);
            } else if matches!(
                path,
                "/api/judge/sync" | "/api/judge" | "/judge/sync" | "/judge"
            ) {
                push_suffixes(&mut candidates, origin);
            }
        }
    } else if primary == "/" || primary.is_empty() {
        push_suffixes(&mut candidates, "");
    } else if matches!(
        primary.as_str(),
        "/api/judge/sync" | "/api/judge" | "/judge/sync" | "/judge"
    ) {
        push_suffixes(&mut candidates, "");
    }

    if let Some(base) = primary.strip_suffix("/api/judge/sync") {
        push_unique(&mut candidates, format!("{base}/api/judge"));
    }

    if let Some(base) = primary.strip_suffix("/judge/sync") {
        push_unique(&mut candidates, format!("{base}/judge"));
    }

    if let Some(base) = primary.strip_suffix("/sync") {
        push_unique(&mut candidates, base.to_string());
    }

    candidates
}

#[cfg(test)]
mod tests {
    use super::endpoint_candidates;

    #[test]
    fn endpoint_candidates_include_common_paths_for_origin() {
        let candidates = endpoint_candidates("http://127.0.0.1:8787");
        assert!(
            candidates
                .iter()
                .any(|c| c == "http://127.0.0.1:8787/api/judge/sync")
        );
        assert!(
            candidates
                .iter()
                .any(|c| c == "http://127.0.0.1:8787/api/judge")
        );
    }

    #[test]
    fn endpoint_candidates_normalize_trailing_slash() {
        let candidates = endpoint_candidates("/api/judge/sync/");
        assert!(candidates.iter().any(|c| c == "/api/judge/sync"));
        assert!(candidates.iter().any(|c| c == "/api/judge"));
    }
}

#[cfg(target_arch = "wasm32")]
fn default_endpoint() -> String {
    endpoint_from_build_env()
        .or_else(endpoint_from_querystring)
        .or_else(endpoint_from_meta)
        .or_else(endpoint_from_local_storage)
        .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string())
}

#[cfg(target_arch = "wasm32")]
fn normalize_endpoint(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(target_arch = "wasm32")]
fn endpoint_from_build_env() -> Option<String> {
    option_env!("SUMMER_QUIZ_JUDGE_ENDPOINT").and_then(normalize_endpoint)
}

#[cfg(target_arch = "wasm32")]
fn endpoint_from_querystring() -> Option<String> {
    let window = web_sys::window()?;
    let search = window.location().search().ok()?;
    let query = search.strip_prefix('?').unwrap_or(search.as_str());

    for pair in query.split('&') {
        let (key, value) = match pair.split_once('=') {
            Some((k, v)) => (k, v),
            None => (pair, ""),
        };

        if key == "judge_endpoint" {
            let decoded = js_sys::decode_uri_component(value).ok()?;
            let decoded = decoded.as_string()?;
            return normalize_endpoint(&decoded);
        }
    }

    None
}

#[cfg(target_arch = "wasm32")]
fn endpoint_from_meta() -> Option<String> {
    let window = web_sys::window()?;
    let document = window.document()?;
    let meta = document
        .query_selector("meta[name='summer-quiz-judge-endpoint']")
        .ok()??;

    meta.get_attribute("content")
        .as_deref()
        .and_then(normalize_endpoint)
}

#[cfg(target_arch = "wasm32")]
fn endpoint_from_local_storage() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    storage
        .get_item("summer_quiz_judge_endpoint")
        .ok()?
        .as_deref()
        .and_then(normalize_endpoint)
}

#[cfg(not(target_arch = "wasm32"))]
fn default_endpoint() -> String {
    std::env::var("SUMMER_QUIZ_JUDGE_ENDPOINT")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_NATIVE_ENDPOINT.to_string())
}

fn to_remote_language(lang: Language) -> &'static str {
    match lang {
        Language::C => "c",
        Language::Pseudocode => "pseudocode",
        Language::Kotlin => "kotlin",
        Language::Java => "java",
        Language::Rust => "rust",
        Language::Python => "python",
        Language::GitGithub => "git_github",
    }
}

fn map_response(resp: JudgeResponse) -> JudgeResult {
    match resp {
        JudgeResponse::Accepted => JudgeResult::Accepted,
        JudgeResponse::CompileError { stderr } => JudgeResult::CompileError { stderr },
        JudgeResponse::WrongAnswer {
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
        JudgeResponse::Timeout {
            test_index,
            input,
            timeout_ms,
        } => JudgeResult::Timeout {
            test_index,
            input,
            timeout_ms,
        },
        JudgeResponse::RuntimeError {
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
        JudgeResponse::InfrastructureError { message } => {
            JudgeResult::InfrastructureError { message }
        }
    }
}

fn build_request(question: &Question, source: &str) -> JudgeRequest {
    JudgeRequest {
        language: to_remote_language(question.language).to_string(),
        source: source.to_string(),
        tests: question.tests.clone(),
        harness: question.judge_harness.clone(),
        question_id: question.id.clone(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn grade_remote_question(question: &Question, user_code: &str) -> JudgeResult {
    let endpoint = endpoint_for(question);
    let payload = build_request(question, user_code);
    let client = reqwest::blocking::Client::new();

    let endpoints = endpoint_candidates(&endpoint);
    let mut last_http_error = None;

    for candidate in endpoints {
        let response = match client.post(&candidate).json(&payload).send() {
            Ok(response) => response,
            Err(err) => {
                return JudgeResult::InfrastructureError {
                    message: format!("Error conectando con judge remoto: {err}"),
                };
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            last_http_error = Some(format!(
                "Judge remoto devolvió HTTP {} en {}{}",
                status,
                candidate,
                if body.trim().is_empty() {
                    String::new()
                } else {
                    format!(". Body: {}", body.trim())
                }
            ));

            if matches!(status.as_u16(), 404 | 405) {
                continue;
            }

            return JudgeResult::InfrastructureError {
                message: last_http_error
                    .unwrap_or_else(|| "Judge remoto devolvió un error HTTP.".to_string()),
            };
        }

        return match response.json::<JudgeResponse>() {
            Ok(body) => map_response(body),
            Err(err) => JudgeResult::InfrastructureError {
                message: format!("Respuesta JSON inválida del judge remoto: {err}"),
            },
        };
    }

    JudgeResult::InfrastructureError {
        message: last_http_error
            .unwrap_or_else(|| "Judge remoto no respondió correctamente.".to_string()),
    }
}

#[cfg(target_arch = "wasm32")]
pub async fn grade_remote_question(question: &Question, user_code: &str) -> JudgeResult {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let endpoint = endpoint_for(question);
    let payload = build_request(question, user_code);
    let payload_json = match serde_json::to_string(&payload) {
        Ok(v) => v,
        Err(err) => {
            return JudgeResult::InfrastructureError {
                message: format!("No se pudo serializar payload del judge remoto: {err}"),
            };
        }
    };

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&JsValue::from_str(&payload_json));

    let window = match web_sys::window() {
        Some(w) => w,
        None => {
            return JudgeResult::InfrastructureError {
                message: "No existe window en entorno WASM.".into(),
            };
        }
    };

    let endpoints = endpoint_candidates(&endpoint);
    let mut last_http_error = None;

    for candidate in endpoints {
        let request = match Request::new_with_str_and_init(&candidate, &opts) {
            Ok(r) => r,
            Err(err) => {
                return JudgeResult::InfrastructureError {
                    message: format!("No se pudo crear request fetch: {:?}", err),
                };
            }
        };

        if let Err(err) = request.headers().set("Content-Type", "application/json") {
            return JudgeResult::InfrastructureError {
                message: format!("No se pudo asignar headers del request: {:?}", err),
            };
        }

        let resp_value = match JsFuture::from(window.fetch_with_request(&request)).await {
            Ok(v) => v,
            Err(err) => {
                return JudgeResult::InfrastructureError {
                    message: format!("Fetch judge remoto falló: {:?}", err),
                };
            }
        };

        let response: Response = match resp_value.dyn_into() {
            Ok(r) => r,
            Err(_) => {
                return JudgeResult::InfrastructureError {
                    message: "La respuesta fetch no es un Response válido.".into(),
                };
            }
        };

        let text_js = match response.text() {
            Ok(promise) => JsFuture::from(promise).await,
            Err(err) => Err(err),
        };

        let text = match text_js.and_then(|v| {
            v.as_string()
                .ok_or_else(|| JsValue::from_str("response.text() no devolvió string"))
        }) {
            Ok(text) => text,
            Err(err) => {
                return JudgeResult::InfrastructureError {
                    message: format!(
                        "No se pudo leer body de respuesta del judge remoto: {:?}",
                        err
                    ),
                };
            }
        };

        if !response.ok() {
            last_http_error = Some(format!(
                "Judge remoto devolvió HTTP {} en {}{}",
                response.status(),
                candidate,
                if text.trim().is_empty() {
                    String::new()
                } else {
                    format!(". Body: {}", text.trim())
                }
            ));

            if matches!(response.status(), 404 | 405) {
                continue;
            }

            return JudgeResult::InfrastructureError {
                message: last_http_error
                    .unwrap_or_else(|| "Judge remoto devolvió un error HTTP.".to_string()),
            };
        }

        return match serde_json::from_str::<JudgeResponse>(&text) {
            Ok(body) => map_response(body),
            Err(err) => JudgeResult::InfrastructureError {
                message: format!("Respuesta JSON inválida del judge remoto: {err}"),
            },
        };
    }

    JudgeResult::InfrastructureError {
        message: last_http_error
            .unwrap_or_else(|| "Judge remoto no respondió correctamente.".to_string()),
    }
}
