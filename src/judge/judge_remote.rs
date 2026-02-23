use crate::judge::judge_c::JudgeResult;
use crate::model::{JudgeTestCase, Language, Question};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Protocol types (shared between client and server)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct JudgeRequest {
    pub language: String,
    pub source: String,
    pub tests: Vec<JudgeTestCase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum JudgeResponse {
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

// ---------------------------------------------------------------------------
// Endpoint resolution
// ---------------------------------------------------------------------------

fn resolve_endpoint(question: &Question) -> Option<String> {
    if let Some(ep) = &question.judge_endpoint {
        let trimmed = ep.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    global_endpoint()
}

#[cfg(not(target_arch = "wasm32"))]
fn global_endpoint() -> Option<String> {
    std::env::var("SUMMER_QUIZ_JUDGE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
fn global_endpoint() -> Option<String> {
    if let Some(url) = option_env!("SUMMER_QUIZ_JUDGE_URL") {
        let trimmed = url.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    if let Some(url) = endpoint_from_meta() {
        return Some(url);
    }

    if let Some(url) = endpoint_from_querystring() {
        return Some(url);
    }

    endpoint_from_local_storage()
}

#[cfg(target_arch = "wasm32")]
fn endpoint_from_meta() -> Option<String> {
    let window = web_sys::window()?;
    let document = window.document()?;
    let meta = document
        .query_selector("meta[name='summer-quiz-judge-url']")
        .ok()??;
    let content = meta.get_attribute("content")?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(target_arch = "wasm32")]
fn endpoint_from_querystring() -> Option<String> {
    let window = web_sys::window()?;
    let search = window.location().search().ok()?;
    let query = search.strip_prefix('?').unwrap_or(search.as_str());

    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if key == "judge_url" {
            let decoded = js_sys::decode_uri_component(value).ok()?;
            let decoded = decoded.as_string()?;
            let trimmed = decoded.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

#[cfg(target_arch = "wasm32")]
fn endpoint_from_local_storage() -> Option<String> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let value = storage.get_item("summer_quiz_judge_url").ok()??;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn language_tag(lang: Language) -> &'static str {
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

fn build_request(question: &Question, source: &str) -> JudgeRequest {
    JudgeRequest {
        language: language_tag(question.language).to_string(),
        source: source.to_string(),
        tests: question.tests.clone(),
        harness: question.judge_harness.clone(),
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

// ---------------------------------------------------------------------------
// Native (blocking) implementation
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
pub fn grade_remote_question(question: &Question, user_code: &str) -> JudgeResult {
    let endpoint = match resolve_endpoint(question) {
        Some(ep) => ep,
        None => {
            return JudgeResult::InfrastructureError {
                message: "No hay endpoint de judge remoto configurado. \
                          Establece la variable de entorno SUMMER_QUIZ_JUDGE_URL."
                    .into(),
            };
        }
    };

    let payload = build_request(question, user_code);
    let client = reqwest::blocking::Client::new();

    let response = match client.post(&endpoint).json(&payload).send() {
        Ok(r) => r,
        Err(err) => {
            return JudgeResult::InfrastructureError {
                message: format!("Error conectando con judge remoto en {endpoint}: {err}"),
            };
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return JudgeResult::InfrastructureError {
            message: format!(
                "Judge remoto devolvió HTTP {status} en {endpoint}. {}",
                body.trim()
            ),
        };
    }

    match response.json::<JudgeResponse>() {
        Ok(body) => map_response(body),
        Err(err) => JudgeResult::InfrastructureError {
            message: format!("Respuesta JSON inválida del judge remoto: {err}"),
        },
    }
}

// ---------------------------------------------------------------------------
// WASM (async) implementation
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
pub async fn grade_remote_question(question: &Question, user_code: &str) -> JudgeResult {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let endpoint = match resolve_endpoint(question) {
        Some(ep) => ep,
        None => {
            return JudgeResult::InfrastructureError {
                message: "No hay endpoint de judge remoto configurado. \
                          Añade <meta name=\"summer-quiz-judge-url\" \
                          content=\"https://tu-server/api/judge\"> en index.html, \
                          o pasa ?judge_url=... en la URL."
                    .into(),
            };
        }
    };

    let payload = build_request(question, user_code);
    let payload_json = match serde_json::to_string(&payload) {
        Ok(v) => v,
        Err(err) => {
            return JudgeResult::InfrastructureError {
                message: format!("No se pudo serializar payload del judge: {err}"),
            };
        }
    };

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&JsValue::from_str(&payload_json));

    let request = match Request::new_with_str_and_init(&endpoint, &opts) {
        Ok(r) => r,
        Err(err) => {
            return JudgeResult::InfrastructureError {
                message: format!("No se pudo crear request fetch para {endpoint}: {err:?}"),
            };
        }
    };

    if let Err(err) = request.headers().set("Content-Type", "application/json") {
        return JudgeResult::InfrastructureError {
            message: format!("No se pudo asignar Content-Type: {err:?}"),
        };
    }

    let window = match web_sys::window() {
        Some(w) => w,
        None => {
            return JudgeResult::InfrastructureError {
                message: "No existe window en entorno WASM.".into(),
            };
        }
    };

    let resp_value = match JsFuture::from(window.fetch_with_request(&request)).await {
        Ok(v) => v,
        Err(err) => {
            return JudgeResult::InfrastructureError {
                message: format!(
                    "Fetch al judge remoto falló ({endpoint}): {err:?}. \
                     Verifica que el server esté corriendo y accesible por HTTPS."
                ),
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

    let text = match response.text() {
        Ok(promise) => match JsFuture::from(promise).await {
            Ok(v) => v.as_string().unwrap_or_default(),
            Err(err) => {
                return JudgeResult::InfrastructureError {
                    message: format!("No se pudo leer body de respuesta: {err:?}"),
                };
            }
        },
        Err(err) => {
            return JudgeResult::InfrastructureError {
                message: format!("No se pudo obtener body de respuesta: {err:?}"),
            };
        }
    };

    if !response.ok() {
        return JudgeResult::InfrastructureError {
            message: format!(
                "Judge remoto devolvió HTTP {} en {endpoint}. {}",
                response.status(),
                text.trim()
            ),
        };
    }

    match serde_json::from_str::<JudgeResponse>(&text) {
        Ok(body) => map_response(body),
        Err(err) => JudgeResult::InfrastructureError {
            message: format!(
                "Respuesta JSON inválida del judge remoto: {err}. Body: {}",
                text.trim()
            ),
        },
    }
}
