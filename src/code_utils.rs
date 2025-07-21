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