use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use summer_quiz::judge::{
    judge_c::{grade_c_question, JudgeResult},
    judge_java::grade_java_question,
    judge_kt::grade_kotlin_question,
    judge_python::grade_python_question,
    judge_rust::grade_rust_question,
};
use summer_quiz::model::{GradingMode, JudgeTestCase, Language, Question};

// ---------------------------------------------------------------------------
// Protocol types (mirror of judge_remote.rs)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct JudgeRequest {
    language: String,
    source: String,
    tests: Vec<JudgeTestCase>,
    harness: Option<String>,
}

#[derive(Debug, Serialize)]
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

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let bind = std::env::var("JUDGE_BIND").unwrap_or_else(|_| "0.0.0.0:8787".to_string());

    let app = Router::new()
        .route("/api/judge", post(handle_judge))
        .route("/health", get(|| async { "ok" }))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .unwrap_or_else(|e| panic!("No se pudo abrir {bind}: {e}"));

    println!("summer_quiz judge server escuchando en http://{bind}");

    axum::serve(listener, app)
        .await
        .expect("server error");
}

async fn handle_judge(
    Json(payload): Json<JudgeRequest>,
) -> Result<Json<JudgeResponse>, (StatusCode, String)> {
    let result = tokio::task::spawn_blocking(move || evaluate(payload))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Task panicked: {e}"),
            )
        })?;

    Ok(Json(result))
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

fn evaluate(payload: JudgeRequest) -> JudgeResponse {
    let question = match build_question(&payload) {
        Ok(q) => q,
        Err(message) => return JudgeResponse::InfrastructureError { message },
    };

    let result = match question.mode {
        Some(GradingMode::JudgeC) => grade_c_question(&question, &payload.source),
        Some(GradingMode::JudgeKotlin) => grade_kotlin_question(&question, &payload.source),
        Some(GradingMode::JudgeJava) => grade_java_question(&question, &payload.source),
        Some(GradingMode::JudgeRust) => grade_rust_question(&question, &payload.source),
        Some(GradingMode::JudgePython) => grade_python_question(&question, &payload.source),
        _ => JudgeResult::InfrastructureError {
            message: "Lenguaje no soportado por el judge server.".into(),
        },
    };

    map_result(result)
}

fn build_question(payload: &JudgeRequest) -> Result<Question, String> {
    if payload.tests.is_empty() {
        return Err("Se requiere al menos un test para evaluar.".into());
    }

    let (language, mode) = match payload.language.trim().to_ascii_lowercase().as_str() {
        "c" => (Language::C, GradingMode::JudgeC),
        "kotlin" => (Language::Kotlin, GradingMode::JudgeKotlin),
        "java" => (Language::Java, GradingMode::JudgeJava),
        "rust" => (Language::Rust, GradingMode::JudgeRust),
        "python" => (Language::Python, GradingMode::JudgePython),
        other => return Err(format!("Lenguaje no soportado: {other}")),
    };

    Ok(Question {
        language,
        module: 0,
        prompt: String::new(),
        answer: String::new(),
        hint: None,
        number: 0,
        input_prefill: None,
        mode: Some(mode),
        tests: payload.tests.clone(),
        judge_harness: payload.harness.clone(),
        judge_endpoint: None,
        is_done: false,
        saw_solution: false,
        attempts: 0,
        fails: 0,
        skips: 0,
        id: None,
    })
}

fn map_result(result: JudgeResult) -> JudgeResponse {
    match result {
        JudgeResult::Accepted => JudgeResponse::Accepted,
        JudgeResult::CompileError { stderr } => JudgeResponse::CompileError { stderr },
        JudgeResult::WrongAnswer {
            test_index,
            input,
            expected,
            received,
            diff,
        } => JudgeResponse::WrongAnswer {
            test_index,
            input,
            expected,
            received,
            diff,
        },
        JudgeResult::Timeout {
            test_index,
            input,
            timeout_ms,
        } => JudgeResponse::Timeout {
            test_index,
            input,
            timeout_ms,
        },
        JudgeResult::RuntimeError {
            test_index,
            input,
            stderr,
            exit_code,
        } => JudgeResponse::RuntimeError {
            test_index,
            input,
            stderr,
            exit_code,
        },
        JudgeResult::InfrastructureError { message } => {
            JudgeResponse::InfrastructureError { message }
        }
    }
}
