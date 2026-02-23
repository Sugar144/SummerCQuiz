use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use serde::{Deserialize, Serialize};
use summer_quiz::judge::{
    judge_c::{JudgeResult, grade_c_question},
    judge_java::grade_java_question,
    judge_kt::grade_kotlin_question,
    judge_python::grade_python_question,
    judge_rust::grade_rust_question,
};
use summer_quiz::model::{GradingMode, JudgeTestCase, Language, Question};

#[derive(Debug, Deserialize)]
struct JudgeRequest {
    language: String,
    source: String,
    tests: Vec<JudgeTestCase>,
    harness: Option<String>,
    question_id: Option<String>,
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

fn main() {
    let bind = std::env::var("JUDGE_BIND").unwrap_or_else(|_| "0.0.0.0:8787".to_string());
    let listener = TcpListener::bind(&bind).expect("no se pudo abrir el puerto del judge server");

    println!("summer_quiz judge server escuchando en http://{bind}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(err) = handle_connection(stream) {
                    eprintln!("error en conexión judge: {err}");
                }
            }
            Err(err) => eprintln!("error aceptando conexión: {err}"),
        }
    }
}

fn handle_connection(mut stream: TcpStream) -> Result<(), String> {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;

    let mut buffer = [0_u8; 64 * 1024];
    let n = stream
        .read(&mut buffer)
        .map_err(|e| format!("no se pudo leer request: {e}"))?;

    let request = String::from_utf8_lossy(&buffer[..n]);
    let mut lines = request.lines();
    let first_line = lines.next().ok_or_else(|| "request vacío".to_string())?;

    if first_line.starts_with("OPTIONS ") {
        write_response(&mut stream, 204, "", "text/plain");
        return Ok(());
    }

    if first_line.starts_with("GET /health") {
        write_response(&mut stream, 200, "ok", "text/plain");
        return Ok(());
    }

    if !is_supported_judge_route(first_line) {
        write_response(&mut stream, 404, "not found", "text/plain");
        return Ok(());
    }

    let body = request
        .split("\r\n\r\n")
        .nth(1)
        .ok_or_else(|| "faltó body JSON".to_string())?;

    let payload: JudgeRequest = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let response = evaluate(payload);
    let response_json = serde_json::to_string(&response).map_err(|e| e.to_string())?;

    write_response(&mut stream, 200, &response_json, "application/json");
    Ok(())
}

fn is_supported_judge_route(first_line: &str) -> bool {
    [
        "POST /api/judge/sync",
        "POST /api/judge",
        "POST /judge/sync",
        "POST /judge",
    ]
    .iter()
    .any(|prefix| first_line.starts_with(prefix))
}

fn evaluate(payload: JudgeRequest) -> JudgeResponse {
    let question = match map_request_to_question(&payload) {
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
            message: "Lenguaje no soportado por el judge remoto.".into(),
        },
    };

    map_result(result)
}

fn map_request_to_question(payload: &JudgeRequest) -> Result<Question, String> {
    if payload.tests.is_empty() {
        return Err("Se requiere al menos un test para evaluar en remoto.".into());
    }

    let (language, mode) = match payload.language.trim().to_ascii_lowercase().as_str() {
        "c" => (Language::C, GradingMode::JudgeC),
        "kotlin" => (Language::Kotlin, GradingMode::JudgeKotlin),
        "java" => (Language::Java, GradingMode::JudgeJava),
        "rust" => (Language::Rust, GradingMode::JudgeRust),
        "python" => (Language::Python, GradingMode::JudgePython),
        other => return Err(format!("Lenguaje remoto no soportado: {other}")),
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
        id: payload.question_id.clone(),
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

fn write_response(stream: &mut TcpStream, status: u16, body: &str, content_type: &str) {
    let status_text = match status {
        200 => "OK",
        204 => "No Content",
        404 => "Not Found",
        _ => "OK",
    };

    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: {content_type}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: POST, GET, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );

    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}
