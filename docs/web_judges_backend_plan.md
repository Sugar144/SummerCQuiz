# Web Judges - Plan de implementación

## Objetivo
Habilitar en WASM la evaluación multirespuesta usando un backend de judges remoto.

## Estado actual
- Los judges locales (C/Java/Kotlin/Rust/Python) requieren `std::process` y filesystem local.
- En WASM devuelven `InfrastructureError` porque el navegador no puede ejecutar compiladores del sistema.
- Se implementó una primera versión de `judge_remote` para **native** (desktop) que consume HTTP síncrono y sirve como contrato base del backend.

## Contrato HTTP inicial
Endpoint por defecto (override por pregunta):
- `POST http://127.0.0.1:8787/api/judge/sync`

Payload (`application/json`):
```json
{
  "language": "c|pseudocode|kotlin|java|rust|python|git_github",
  "source": "...",
  "tests": [{"input": "...", "output": "..."}],
  "harness": "... opcional ...",
  "question_id": "... opcional ..."
}
```

Respuesta esperada (`status` tagged union):
- `accepted`
- `compile_error` + `stderr`
- `wrong_answer` + `test_index,input,expected,received,diff`
- `timeout` + `test_index,input,timeout_ms`
- `runtime_error` + `test_index,input,stderr,exit_code`
- `infrastructure_error` + `message`

## Diseño propuesto para WASM (fase siguiente)
1. API backend asíncrona:
   - `POST /api/judge/submit` -> crea ejecución.
   - `GET /api/judge/result/:id` -> obtiene resultado.
2. Frontend web:
   - Si la pregunta usa `mode: judge_remote`, envía código y muestra estado "evaluando...".
   - Hace polling no bloqueante y aplica el `JudgeResult` al terminar.
3. Seguridad backend:
   - Sandbox por ejecución.
   - Timeout por test.
   - Límite de CPU/Memoria.

## Cambios de código iniciados
- `GradingMode::JudgeRemote` para habilitar migración progresiva por pregunta.
- `Question.judge_endpoint` para permitir override del endpoint remoto por pregunta.
- `src/judge/judge_remote.rs` implementa cliente remoto síncrono (native) + mapeo a `JudgeResult`.

## Próximo paso técnico
Implementar cliente HTTP en WASM (`fetch`) + estado asíncrono en la UI para submit/poll.
