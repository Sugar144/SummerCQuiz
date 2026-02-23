use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use summer_quiz::judge::{
    judge_c::{JudgeResult, grade_c_question},
    judge_java::grade_java_question,
    judge_kt::grade_kotlin_question,
    judge_python::grade_python_question,
    judge_rust::grade_rust_question,
};
use summer_quiz::model::{GradingMode, JudgeTestCase, Language, Question};

const MAX_BODY_BYTES: usize = 1_000_000;

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

struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
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
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;

    let request = match read_http_request(&mut stream) {
        Ok(req) => req,
        Err(err) => {
            write_text_response(&mut stream, 400, &format!("bad request: {err}"));
            return Ok(());
        }
    };

    if request.method == "OPTIONS" {
        write_empty_response(&mut stream, 204);
        return Ok(());
    }

    if request.method == "GET" && request.path == "/health" {
        write_text_response(&mut stream, 200, "ok");
        return Ok(());
    }

    if request.method == "POST" && request.path == "/api/judge/sync" {
        return handle_judge_sync(&mut stream, request);
    }

    write_text_response(&mut stream, 404, "not found");
    Ok(())
}

fn handle_judge_sync(stream: &mut TcpStream, request: HttpRequest) -> Result<(), String> {
    let Some(content_type) = request.header("content-type") else {
        write_json_response(
            stream,
            400,
            &JudgeResponse::InfrastructureError {
                message: "Falta header Content-Type.".into(),
            },
        );
        return Ok(());
    };

    if !content_type
        .to_ascii_lowercase()
        .contains("application/json")
    {
        write_json_response(
            stream,
            415,
            &JudgeResponse::InfrastructureError {
                message: "Solo se acepta Content-Type: application/json.".into(),
            },
        );
        return Ok(());
    }

    let payload: JudgeRequest = match serde_json::from_slice(&request.body) {
        Ok(v) => v,
        Err(err) => {
            write_json_response(
                stream,
                400,
                &JudgeResponse::InfrastructureError {
                    message: format!("JSON inválido: {err}"),
                },
            );
            return Ok(());
        }
    };

    let response = evaluate(payload);
    write_json_response(stream, 200, &response);
    Ok(())
}

fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    let mut buffer = Vec::with_capacity(4096);
    let mut temp = [0_u8; 1024];

    loop {
        let n = stream
            .read(&mut temp)
            .map_err(|e| format!("no se pudo leer request: {e}"))?;
        if n == 0 {
            break;
        }
        buffer.extend_from_slice(&temp[..n]);

        if find_header_end(&buffer).is_some() {
            break;
        }

        if buffer.len() > MAX_BODY_BYTES {
            return Err("headers demasiado grandes".into());
        }
    }

    let header_end = find_header_end(&buffer).ok_or_else(|| "headers incompletos".to_string())?;
    let header_bytes = &buffer[..header_end];
    let header_text =
        std::str::from_utf8(header_bytes).map_err(|_| "headers no son UTF-8 válido".to_string())?;

    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| "faltó request line".to_string())?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| "faltó método HTTP".to_string())?
        .to_string();
    let path = parts
        .next()
        .ok_or_else(|| "faltó path HTTP".to_string())?
        .to_string();

    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let mut kv = line.splitn(2, ':');
        let key = kv
            .next()
            .ok_or_else(|| "header inválido".to_string())?
            .trim()
            .to_ascii_lowercase();
        let value = kv
            .next()
            .ok_or_else(|| "header inválido (sin ':')".to_string())?
            .trim()
            .to_string();
        headers.insert(key, value);
    }

    let mut body = buffer[(header_end + 4)..].to_vec();
    let expected_len = headers
        .get("content-length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);

    if expected_len > MAX_BODY_BYTES {
        return Err("body demasiado grande".into());
    }

    while body.len() < expected_len {
        let n = stream
            .read(&mut temp)
            .map_err(|e| format!("no se pudo leer body: {e}"))?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&temp[..n]);
    }

    if body.len() < expected_len {
        return Err("body incompleto".into());
    }
    body.truncate(expected_len);

    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
    })
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|w| w == b"\r\n\r\n")
}

impl HttpRequest {
    fn header(&self, key: &str) -> Option<&str> {
        self.headers
            .get(&key.to_ascii_lowercase())
            .map(|s| s.as_str())
    }
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

fn write_empty_response(stream: &mut TcpStream, status: u16) {
    write_http_response(stream, status, "text/plain", "")
}

fn write_text_response(stream: &mut TcpStream, status: u16, body: &str) {
    write_http_response(stream, status, "text/plain; charset=utf-8", body)
}

fn write_json_response(stream: &mut TcpStream, status: u16, body: &JudgeResponse) {
    match serde_json::to_string(body) {
        Ok(json) => write_http_response(stream, status, "application/json", &json),
        Err(err) => write_http_response(
            stream,
            500,
            "text/plain; charset=utf-8",
            &format!("error serializando respuesta JSON: {err}"),
        ),
    }
}

fn write_http_response(stream: &mut TcpStream, status: u16, content_type: &str, body: &str) {
    let status_text = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        415 => "Unsupported Media Type",
        500 => "Internal Server Error",
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
