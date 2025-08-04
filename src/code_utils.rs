use egui_code_editor::Syntax;

pub fn normalize_code(input: &str) -> String {
    let mut code = String::new();
    let mut in_block_comment = false;
    for line in input.lines() {
        let mut line = line;
        // Eliminar comentarios de bloque que empiezan en esta línea
        if !in_block_comment {
            if let Some(start) = line.find("/*") {
                in_block_comment = true;
                line = &line[..start];
            }
        }
        // Eliminar comentarios de línea //
        if !in_block_comment {
            if let Some(start) = line.find("//") {
                line = &line[..start];
            }
        }
        // Si estamos dentro de un bloque /* ... */
        if in_block_comment {
            if let Some(end) = line.find("*/") {
                in_block_comment = false;
                line = &line[(end + 2)..];
            } else {
                continue; // línea completamente comentada
            }
        }
        // Añadir línea si queda algo útil
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            code.push_str(trimmed);
        }
    }
    code.replace(char::is_whitespace, "")
}

pub fn c_syntax() -> Syntax {
    Syntax::new("c")
        .with_comment("//")
        .with_comment_multiline(["/*", "*/"])
        .with_keywords([
            "int", "char", "void", "if", "else", "for", "while", "return", "break", "continue",
            "switch", "case", "default", "struct", "typedef", "enum", "union", "sizeof", "do",
            "goto", "static", "const", "volatile", "unsigned", "signed", "short", "long", "float",
            "double", "auto", "extern", "register",
        ])
        .with_types([
            "int", "char", "float", "double", "void", "size_t", "uint8_t", "uint16_t", "uint32_t", "uint64_t",
        ])
}

pub fn pseudo_syntax() -> Syntax {
    Syntax::new("c")
        .with_comment("//")
        .with_comment_multiline(["/*", "*/"])
        .with_keywords([
            "end", "const", "var", "type", "record", "for", "to", "while", "if", "then","else",
            "switch", "case", "default", "function", "action", "algorithm", "vector", "of"
        ])
        .with_types([
            "integer", "character", "real", "string", "boolean"
        ])
}