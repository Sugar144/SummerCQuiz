pub fn normalize_newlines(value: &str) -> String {
    value.replace("\r\n", "\n")
}

pub fn matches_expected_output(received: &str, expected: &str) -> bool {
    if received == expected {
        return true;
    }

    let received_trimmed = received.trim_end();
    let expected_trimmed = expected.trim_end();

    if received_trimmed == expected_trimmed {
        return true;
    }

    if expected_trimmed.is_empty() {
        return received_trimmed.is_empty();
    }

    if received_trimmed.ends_with(expected_trimmed) {
        return true;
    }

    if !expected_trimmed.chars().any(char::is_whitespace) {
        if let (Some(exp_tok), Some(rec_tok)) = (
            last_meaningful_token(expected_trimmed),
            last_meaningful_token(received_trimmed),
        ) {
            return rec_tok == exp_tok;
        }
    }

    false
}

pub fn last_meaningful_token(text: &str) -> Option<String> {
    text.split_whitespace().rev().find_map(|raw| {
        let token = raw.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-');
        if token.is_empty() {
            None
        } else {
            Some(token.to_string())
        }
    })
}

pub fn line_diff(expected: &str, received: &str) -> String {
    let expected_norm = expected.replace("\r\n", "\n");
    let received_norm = received.replace("\r\n", "\n");
    let exp: Vec<&str> = expected_norm.split('\n').collect();
    let rec: Vec<&str> = received_norm.split('\n').collect();
    let max_lines = exp.len().max(rec.len());

    for i in 0..max_lines {
        let e = exp.get(i).copied().unwrap_or("<sin línea>");
        let r = rec.get(i).copied().unwrap_or("<sin línea>");
        if e != r {
            return format!("Línea {}\n- esperado: {:?}\n+ recibido: {:?}", i + 1, e, r);
        }
    }

    "Diferencia no localizada (posible carácter invisible).".into()
}