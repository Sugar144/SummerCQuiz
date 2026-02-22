use crate::judge::judge_c::JudgeResult;
use crate::model::{GradingMode, JudgeTestCase, Language, Question};

#[derive(Debug, Clone)]
pub enum PseudoError {
    LexError {
        message: String,
        line: usize,
        col: usize,
    },
    ParseError {
        message: String,
        line: usize,
        col: usize,
    },
    UnsupportedFeature {
        feature: String,
        line: usize,
        col: usize,
    },
    TranspileError {
        message: String,
    },
}

#[derive(Debug, Clone)]
pub struct PseudoConfig {
    pub double_type: &'static str,
}

impl Default for PseudoConfig {
    fn default() -> Self {
        Self {
            double_type: "double",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CJudge;

impl CJudge {
    pub fn grade(&self, question: &Question, code: &str) -> JudgeResult {
        crate::judge::judge_c::grade_c_question(question, code)
    }
}

pub fn run_pseudo_tests(
    code: &str,
    tests: &[JudgeTestCase],
    cfg: &PseudoConfig,
    c_judge: &CJudge,
) -> JudgeResult {
    if tests.is_empty() {
        return JudgeResult::InfrastructureError {
            message: "La pregunta judge_pseudo no tiene tests configurados.".into(),
        };
    }

    let c_code = match pseudo_to_c_with_config(code, cfg) {
        Ok(code) => code,
        Err(err) => {
            return JudgeResult::CompileError {
                stderr: format_pseudo_error(&err),
            };
        }
    };

    let question = Question {
        language: Language::C,
        module: 0,
        prompt: String::new(),
        answer: c_code,
        hint: None,
        number: 0,
        input_prefill: None,
        mode: Some(GradingMode::JudgeC),
        tests: tests.to_vec(),
        judge_harness: None,
        is_done: false,
        saw_solution: false,
        attempts: 0,
        fails: 0,
        skips: 0,
        id: None,
    };

    c_judge.grade(&question, &question.answer)
}

pub fn pseudo_to_c(code: &str) -> Result<String, PseudoError> {
    pseudo_to_c_with_config(code, &PseudoConfig::default())
}

fn pseudo_to_c_with_config(code: &str, cfg: &PseudoConfig) -> Result<String, PseudoError> {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.lex()?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;
    transpile_program(&program, cfg)
}

fn format_pseudo_error(err: &PseudoError) -> String {
    match err {
        PseudoError::LexError { message, line, col } => {
            format!("LexError [{line}:{col}]: {message}")
        },
        PseudoError::ParseError { message, line, col } => {
            format!("ParseError [{line}:{col}]: {message}")
        },
        PseudoError::UnsupportedFeature { feature, line, col } => {
            format!("UnsupportedFeature [{line}:{col}]: {feature}")
        },
        PseudoError::TranspileError { message } => format!("TranspileError: {message}"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenKind {
    Keyword(String),
    Identifier(String),
    Number(String),
    StringLiteral(String),
    Symbol(char),
    Assign,
    LessEq,
    GreaterEq,
    NotEq,
    Eof,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    line: usize,
    col: usize,
}

struct Lexer<'a> {
    chars: Vec<char>,
    idx: usize,
    line: usize,
    col: usize,
    _src: &'a str,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            chars: src.chars().collect(),
            idx: 0,
            line: 1,
            col: 1,
            _src: src,
        }
    }

    fn lex(&mut self) -> Result<Vec<Token>, PseudoError> {
        let mut out = Vec::new();
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.bump();
                continue;
            }
            if ch == '{' {
                self.consume_comment()?;
                continue;
            }
            let line = self.line;
            let col = self.col;

            let kind = if ch.is_ascii_alphabetic() || ch == '_' {
                self.lex_word()
            } else if ch.is_ascii_digit() {
                self.lex_number()
            } else {
                match ch {
                    '"' => self.lex_string()?,
                    ':' => {
                        self.bump();
                        if self.peek() == Some('=') {
                            self.bump();
                            TokenKind::Assign
                        } else {
                            TokenKind::Symbol(':')
                        }
                    }
                    '≤' => {
                        self.bump();
                        TokenKind::LessEq
                    }
                    '≥' => {
                        self.bump();
                        TokenKind::GreaterEq
                    }
                    '≠' => {
                        self.bump();
                        TokenKind::NotEq
                    }
                    '<' => {
                        self.bump();
                        if self.peek() == Some('=') {
                            self.bump();
                            TokenKind::LessEq
                        } else if self.peek() == Some('>') {
                            self.bump();
                            TokenKind::NotEq
                        } else {
                            TokenKind::Symbol('<')
                        }
                    }
                    '>' => {
                        self.bump();
                        if self.peek() == Some('=') {
                            self.bump();
                            TokenKind::GreaterEq
                        } else {
                            TokenKind::Symbol('>')
                        }
                    }
                    '=' | '+' | '-' | '*' | '/' | '%' | '(' | ')' | ',' | ';' => {
                        self.bump();
                        TokenKind::Symbol(ch)
                    }
                    _ => {
                        return Err(PseudoError::LexError {
                            message: format!("Símbolo no soportado: {ch}"),
                            line,
                            col,
                        });
                    }
                }
            };

            out.push(Token { kind, line, col });
        }
        out.push(Token {
            kind: TokenKind::Eof,
            line: self.line,
            col: self.col,
        });
        Ok(out)
    }

    fn consume_comment(&mut self) -> Result<(), PseudoError> {
        let line = self.line;
        let col = self.col;
        self.bump();
        while let Some(ch) = self.peek() {
            if ch == '}' {
                self.bump();
                return Ok(());
            }
            self.bump();
        }
        Err(PseudoError::LexError {
            message: "Comentario sin cerrar".into(),
            line,
            col,
        })
    }

    fn lex_word(&mut self) -> TokenKind {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                s.push(ch);
                self.bump();
            } else {
                break;
            }
        }
        let lower = s.to_ascii_lowercase();
        if is_keyword(&lower) {
            TokenKind::Keyword(lower)
        } else {
            TokenKind::Identifier(s)
        }
    }

    fn lex_number(&mut self) -> TokenKind {
        let mut s = String::new();
        let mut seen_dot = false;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                s.push(ch);
                self.bump();
            } else if ch == '.' && !seen_dot {
                seen_dot = true;
                s.push(ch);
                self.bump();
            } else {
                break;
            }
        }
        TokenKind::Number(s)
    }

    fn lex_string(&mut self) -> Result<TokenKind, PseudoError> {
        self.bump();
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch == '"' {
                self.bump();
                return Ok(TokenKind::StringLiteral(s));
            }
            s.push(ch);
            self.bump();
        }
        Err(PseudoError::LexError {
            message: "String sin cerrar".into(),
            line: self.line,
            col: self.col,
        })
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.idx).copied()
    }

    fn bump(&mut self) {
        if let Some(ch) = self.peek() {
            self.idx += 1;
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
    }
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "algorithm"
            | "end"
            | "var"
            | "const"
            | "if"
            | "then"
            | "else"
            | "while"
            | "do"
            | "for"
            | "to"
            | "step"
            | "function"
            | "action"
            | "return"
            | "integer"
            | "real"
            | "boolean"
            | "char"
            | "string"
            | "true"
            | "false"
            | "and"
            | "or"
            | "not"
            | "in"
            | "out"
            | "inout"
            | "readinteger"
            | "readreal"
            | "readchar"
            | "readstring"
            | "readboolean"
            | "writeinteger"
            | "writereal"
            | "writechar"
            | "writestring"
            | "writeboolean"
    )
}

#[derive(Debug, Clone)]
struct Program {
    declarations: Vec<Decl>,
    subprograms: Vec<Subprogram>,
    body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
enum Decl {
    Var {
        name: String,
        typ: TypeName,
    },
    Const {
        name: String,
        typ: TypeName,
        value: Expr,
    },
}

#[derive(Debug, Clone)]
enum TypeName {
    Integer,
    Real,
    Boolean,
    Char,
    String,
}

#[derive(Debug, Clone)]
enum ParamMode {
    In,
    Out,
    InOut,
}

#[derive(Debug, Clone)]
struct Param {
    mode: ParamMode,
    name: String,
    typ: TypeName,
}

#[derive(Debug, Clone)]
enum Subprogram {
    Function {
        name: String,
        params: Vec<Param>,
        return_type: TypeName,
        body: Vec<Stmt>,
    },
    Action {
        name: String,
        params: Vec<Param>,
        body: Vec<Stmt>,
    },
}

#[derive(Debug, Clone)]
enum Stmt {
    Assign {
        target: String,
        expr: Expr,
    },
    If {
        cond: Expr,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    For {
        var: String,
        start: Expr,
        end: Expr,
        step: Option<Expr>,
        body: Vec<Stmt>,
    },
    Return(Expr),
    ExprOnly(Expr),
}

#[derive(Debug, Clone)]
enum Expr {
    Var(String),
    Number(String),
    Bool(bool),
    StringLiteral(String),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone)]
enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}

struct Parser {
    tokens: Vec<Token>,
    idx: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, idx: 0 }
    }

    fn parse_program(&mut self) -> Result<Program, PseudoError> {
        self.expect_keyword("algorithm")?;
        if self.check_identifier() {
            self.bump();
        }

        let declarations = self.parse_declarations()?;
        let subprograms = self.parse_subprograms()?;
        let body = self.parse_block_until_end_algorithm()?;

        self.expect_keyword("end")?;
        self.expect_keyword("algorithm")?;

        Ok(Program {
            declarations,
            subprograms,
            body,
        })
    }

    fn parse_declarations(&mut self) -> Result<Vec<Decl>, PseudoError> {
        let mut out = Vec::new();
        loop {
            if self.consume_keyword("var") {
                while !self.consume_two_keywords("end", "var") {
                    let name = self.expect_identifier()?;
                    self.expect_symbol(':')?;
                    let typ = self.parse_type()?;
                    self.expect_symbol(';')?;
                    out.push(Decl::Var { name, typ });
                }
            } else if self.consume_keyword("const") {
                while !self.consume_two_keywords("end", "const") {
                    let name = self.expect_identifier()?;
                    self.expect_symbol(':')?;
                    let typ = self.parse_type()?;
                    self.expect_symbol('=')?;
                    let value = self.parse_expr()?;
                    self.expect_symbol(';')?;
                    out.push(Decl::Const { name, typ, value });
                }
            } else {
                break;
            }
        }
        Ok(out)
    }

    fn parse_subprograms(&mut self) -> Result<Vec<Subprogram>, PseudoError> {
        let mut out = Vec::new();
        loop {
            if self.consume_keyword("function") {
                let name = self.expect_identifier()?;
                let params = self.parse_params()?;
                self.expect_symbol(':')?;
                let return_type = self.parse_type()?;
                let body = self.parse_statements_until_end("function")?;
                out.push(Subprogram::Function {
                    name,
                    params,
                    return_type,
                    body,
                });
            } else if self.consume_keyword("action") {
                let name = self.expect_identifier()?;
                let params = self.parse_params()?;
                let body = self.parse_statements_until_end("action")?;
                out.push(Subprogram::Action { name, params, body });
            } else {
                break;
            }
        }
        Ok(out)
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, PseudoError> {
        self.expect_symbol('(')?;
        let mut out = Vec::new();
        if self.consume_symbol(')') {
            return Ok(out);
        }

        loop {
            let mode = if self.consume_keyword("in") {
                ParamMode::In
            } else if self.consume_keyword("out") {
                ParamMode::Out
            } else if self.consume_keyword("inout") {
                ParamMode::InOut
            } else {
                ParamMode::In
            };

            let name = self.expect_identifier()?;
            self.expect_symbol(':')?;
            let typ = self.parse_type()?;
            out.push(Param { mode, name, typ });

            if self.consume_symbol(')') {
                break;
            }
            self.expect_symbol(',')?;
        }

        Ok(out)
    }

    fn parse_block_until_end_algorithm(&mut self) -> Result<Vec<Stmt>, PseudoError> {
        let mut out = Vec::new();
        while !self.peek_is_two_keywords("end", "algorithm") {
            out.push(self.parse_stmt()?);
        }
        Ok(out)
    }

    fn parse_statements_until_end(&mut self, block_name: &str) -> Result<Vec<Stmt>, PseudoError> {
        let mut out = Vec::new();
        while !self.peek_is_two_keywords("end", block_name) {
            out.push(self.parse_stmt()?);
        }
        self.expect_keyword("end")?;
        self.expect_keyword(block_name)?;
        Ok(out)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, PseudoError> {
        if self.consume_keyword("if") {
            let cond = self.parse_expr()?;
            self.expect_keyword("then")?;
            let mut then_body = Vec::new();
            while !self.peek_is_keyword("else") && !self.peek_is_two_keywords("end", "if") {
                then_body.push(self.parse_stmt()?);
            }

            let mut else_body = Vec::new();
            if self.consume_keyword("else") {
                while !self.peek_is_two_keywords("end", "if") {
                    else_body.push(self.parse_stmt()?);
                }
            }
            self.expect_keyword("end")?;
            self.expect_keyword("if")?;
            return Ok(Stmt::If {
                cond,
                then_body,
                else_body,
            });
        }

        if self.consume_keyword("while") {
            let cond = self.parse_expr()?;
            self.expect_keyword("do")?;
            let mut body = Vec::new();
            while !self.peek_is_two_keywords("end", "while") {
                body.push(self.parse_stmt()?);
            }
            self.expect_keyword("end")?;
            self.expect_keyword("while")?;
            return Ok(Stmt::While { cond, body });
        }

        if self.consume_keyword("for") {
            let var = self.expect_identifier()?;
            self.expect_assign()?;
            let start = self.parse_expr()?;
            self.expect_keyword("to")?;
            let end = self.parse_expr()?;
            let step = if self.consume_keyword("step") {
                Some(self.parse_expr()?)
            } else {
                None
            };
            self.expect_keyword("do")?;
            let mut body = Vec::new();
            while !self.peek_is_two_keywords("end", "for") {
                body.push(self.parse_stmt()?);
            }
            self.expect_keyword("end")?;
            self.expect_keyword("for")?;
            return Ok(Stmt::For {
                var,
                start,
                end,
                step,
                body,
            });
        }

        if self.consume_keyword("return") {
            let value = self.parse_expr()?;
            self.consume_symbol(';');
            return Ok(Stmt::Return(value));
        }

        if self.check_identifier() {
            if self.peek_next_is_assign() {
                let target = self.expect_identifier()?;
                self.expect_assign()?;
                let expr = self.parse_expr()?;
                self.consume_symbol(';');
                return Ok(Stmt::Assign { target, expr });
            }

            let expr = self.parse_expr()?;
            self.consume_symbol(';');
            return Ok(Stmt::ExprOnly(expr));
        }

        if self.peek_is_callable_keyword() {
            let expr = self.parse_expr()?;
            self.consume_symbol(';');
            return Ok(Stmt::ExprOnly(expr));
        }

        let tok = self.curr();
        Err(PseudoError::ParseError {
            message: "Sentencia no reconocida".into(),
            line: tok.line,
            col: tok.col,
        })
    }

    fn parse_expr(&mut self) -> Result<Expr, PseudoError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, PseudoError> {
        let mut node = self.parse_and()?;
        while self.consume_keyword("or") {
            let rhs = self.parse_and()?;
            node = Expr::Binary {
                left: Box::new(node),
                op: BinaryOp::Or,
                right: Box::new(rhs),
            };
        }
        Ok(node)
    }

    fn parse_and(&mut self) -> Result<Expr, PseudoError> {
        let mut node = self.parse_cmp()?;
        while self.consume_keyword("and") {
            let rhs = self.parse_cmp()?;
            node = Expr::Binary {
                left: Box::new(node),
                op: BinaryOp::And,
                right: Box::new(rhs),
            };
        }
        Ok(node)
    }

    fn parse_cmp(&mut self) -> Result<Expr, PseudoError> {
        let mut node = self.parse_term()?;
        loop {
            let op = if self.consume_symbol('=') {
                Some(BinaryOp::Eq)
            } else if self.consume_not_eq() {
                Some(BinaryOp::NotEq)
            } else if self.consume_symbol('<') {
                Some(BinaryOp::Lt)
            } else if self.consume_symbol('>') {
                Some(BinaryOp::Gt)
            } else if self.consume_less_eq() {
                Some(BinaryOp::LtEq)
            } else if self.consume_greater_eq() {
                Some(BinaryOp::GtEq)
            } else {
                None
            };

            if let Some(op) = op {
                let rhs = self.parse_term()?;
                node = Expr::Binary {
                    left: Box::new(node),
                    op,
                    right: Box::new(rhs),
                };
            } else {
                break;
            }
        }
        Ok(node)
    }

    fn parse_term(&mut self) -> Result<Expr, PseudoError> {
        let mut node = self.parse_factor()?;
        loop {
            let op = if self.consume_symbol('+') {
                Some(BinaryOp::Add)
            } else if self.consume_symbol('-') {
                Some(BinaryOp::Sub)
            } else {
                None
            };
            if let Some(op) = op {
                let rhs = self.parse_factor()?;
                node = Expr::Binary {
                    left: Box::new(node),
                    op,
                    right: Box::new(rhs),
                };
            } else {
                break;
            }
        }
        Ok(node)
    }

    fn parse_factor(&mut self) -> Result<Expr, PseudoError> {
        let mut node = self.parse_unary()?;
        loop {
            let op = if self.consume_symbol('*') {
                Some(BinaryOp::Mul)
            } else if self.consume_symbol('/') {
                Some(BinaryOp::Div)
            } else {
                None
            };
            if let Some(op) = op {
                let rhs = self.parse_unary()?;
                node = Expr::Binary {
                    left: Box::new(node),
                    op,
                    right: Box::new(rhs),
                };
            } else {
                break;
            }
        }
        Ok(node)
    }

    fn parse_unary(&mut self) -> Result<Expr, PseudoError> {
        if self.consume_keyword("not") {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
            });
        }
        if self.consume_symbol('-') {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
            });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, PseudoError> {
        if self.consume_symbol('(') {
            let expr = self.parse_expr()?;
            self.expect_symbol(')')?;
            return Ok(expr);
        }
        if let Some(n) = self.consume_number() {
            return Ok(Expr::Number(n));
        }
        if let Some(s) = self.consume_string() {
            return Ok(Expr::StringLiteral(s));
        }
        if self.consume_keyword("true") {
            return Ok(Expr::Bool(true));
        }
        if self.consume_keyword("false") {
            return Ok(Expr::Bool(false));
        }

        if self.check_identifier() || self.peek_is_callable_keyword() {
            let ident = self.expect_callable_name()?;
            if self.consume_symbol('(') {
                let mut args = Vec::new();
                if !self.consume_symbol(')') {
                    loop {
                        args.push(self.parse_expr()?);
                        if self.consume_symbol(')') {
                            break;
                        }
                        self.expect_symbol(',')?;
                    }
                }
                return Ok(Expr::Call { name: ident, args });
            }
            return Ok(Expr::Var(ident));
        }

        let tok = self.curr();
        Err(PseudoError::ParseError {
            message: "Expresión no válida".into(),
            line: tok.line,
            col: tok.col,
        })
    }

    fn parse_type(&mut self) -> Result<TypeName, PseudoError> {
        if self.consume_keyword("integer") {
            return Ok(TypeName::Integer);
        }
        if self.consume_keyword("real") {
            return Ok(TypeName::Real);
        }
        if self.consume_keyword("boolean") {
            return Ok(TypeName::Boolean);
        }
        if self.consume_keyword("char") {
            return Ok(TypeName::Char);
        }
        if self.consume_keyword("string") {
            return Ok(TypeName::String);
        }

        let tok = self.curr();
        Err(PseudoError::UnsupportedFeature {
            feature: "Solo se soportan tipos básicos en judge_pseudo MVP".into(),
            line: tok.line,
            col: tok.col,
        })
    }

    fn curr(&self) -> &Token {
        &self.tokens[self.idx]
    }

    fn bump(&mut self) {
        if self.idx + 1 < self.tokens.len() {
            self.idx += 1;
        }
    }

    fn check_identifier(&self) -> bool {
        matches!(self.curr().kind, TokenKind::Identifier(_))
    }

    fn peek_next_is_assign(&self) -> bool {
        matches!(self.curr().kind, TokenKind::Identifier(_))
            && self
                .tokens
                .get(self.idx + 1)
                .map(|t| matches!(t.kind, TokenKind::Assign))
                .unwrap_or(false)
    }

    fn peek_is_callable_keyword(&self) -> bool {
        matches!(&self.curr().kind, TokenKind::Keyword(v) if matches!(v.as_str(),
            "readinteger"|"readreal"|"readchar"|"readstring"|"readboolean"|
            "writeinteger"|"writereal"|"writechar"|"writestring"|"writeboolean"
        ))
    }

    fn expect_callable_name(&mut self) -> Result<String, PseudoError> {
        let tok = self.curr().clone();
        match tok.kind {
            TokenKind::Identifier(name) => {
                self.bump();
                Ok(name)
            }
            TokenKind::Keyword(name) if self.peek_is_callable_keyword() => {
                self.bump();
                Ok(name)
            }
            _ => Err(PseudoError::ParseError {
                message: "Se esperaba llamada o identificador".into(),
                line: tok.line,
                col: tok.col,
            }),
        }
    }
    fn expect_identifier(&mut self) -> Result<String, PseudoError> {
        let tok = self.curr().clone();
        if let TokenKind::Identifier(name) = tok.kind {
            self.bump();
            Ok(name)
        } else {
            Err(PseudoError::ParseError {
                message: "Se esperaba un identificador".into(),
                line: tok.line,
                col: tok.col,
            })
        }
    }

    fn expect_keyword(&mut self, kw: &str) -> Result<(), PseudoError> {
        if self.consume_keyword(kw) {
            Ok(())
        } else {
            let tok = self.curr();
            Err(PseudoError::ParseError {
                message: format!("Se esperaba keyword '{kw}'"),
                line: tok.line,
                col: tok.col,
            })
        }
    }

    fn consume_keyword(&mut self, kw: &str) -> bool {
        if matches!(&self.curr().kind, TokenKind::Keyword(v) if v == kw) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn consume_two_keywords(&mut self, first: &str, second: &str) -> bool {
        if self.peek_is_two_keywords(first, second) {
            self.bump();
            self.bump();
            true
        } else {
            false
        }
    }

    fn peek_is_two_keywords(&self, first: &str, second: &str) -> bool {
        matches!(&self.curr().kind, TokenKind::Keyword(v) if v == first)
            && self
                .tokens
                .get(self.idx + 1)
                .map(|t| matches!(&t.kind, TokenKind::Keyword(v) if v == second))
                .unwrap_or(false)
    }

    fn peek_is_keyword(&self, kw: &str) -> bool {
        matches!(&self.curr().kind, TokenKind::Keyword(v) if v == kw)
    }

    fn expect_symbol(&mut self, sym: char) -> Result<(), PseudoError> {
        if self.consume_symbol(sym) {
            Ok(())
        } else {
            let tok = self.curr();
            Err(PseudoError::ParseError {
                message: format!("Se esperaba símbolo '{sym}'"),
                line: tok.line,
                col: tok.col,
            })
        }
    }

    fn consume_symbol(&mut self, sym: char) -> bool {
        if matches!(self.curr().kind, TokenKind::Symbol(v) if v == sym) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect_assign(&mut self) -> Result<(), PseudoError> {
        if matches!(self.curr().kind, TokenKind::Assign) {
            self.bump();
            Ok(())
        } else {
            let tok = self.curr();
            Err(PseudoError::ParseError {
                message: "Se esperaba ':='".into(),
                line: tok.line,
                col: tok.col,
            })
        }
    }

    fn consume_not_eq(&mut self) -> bool {
        if matches!(self.curr().kind, TokenKind::NotEq) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn consume_less_eq(&mut self) -> bool {
        if matches!(self.curr().kind, TokenKind::LessEq) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn consume_greater_eq(&mut self) -> bool {
        if matches!(self.curr().kind, TokenKind::GreaterEq) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn consume_number(&mut self) -> Option<String> {
        if let TokenKind::Number(v) = self.curr().kind.clone() {
            self.bump();
            Some(v)
        } else {
            None
        }
    }

    fn consume_string(&mut self) -> Option<String> {
        if let TokenKind::StringLiteral(v) = self.curr().kind.clone() {
            self.bump();
            Some(v)
        } else {
            None
        }
    }
}

fn transpile_program(program: &Program, cfg: &PseudoConfig) -> Result<String, PseudoError> {
    let mut out = String::new();
    out.push_str("#include <stdio.h>\n#include <stdbool.h>\n\nint read_integer(void) { int v = 0; scanf(\"%d\", &v); return v; }\ndouble read_real(void) { double v = 0; scanf(\"%lf\", &v); return v; }\nchar read_char(void) { char v = 0; scanf(\" %c\", &v); return v; }\nbool read_boolean(void) { int v = 0; scanf(\"%d\", &v); return v != 0; }\n\n");

    for sub in &program.subprograms {
        transpile_subprogram(sub, cfg, &mut out)?;
        out.push('\n');
    }

    out.push_str("int main(void) {\n");
    for decl in &program.declarations {
        transpile_decl(decl, cfg, &mut out, 1)?;
    }
    for stmt in &program.body {
        transpile_stmt(stmt, &mut out, 1, &[])?;
    }
    out.push_str("    return 0;\n}\n");

    Ok(out)
}

fn transpile_subprogram(
    sub: &Subprogram,
    cfg: &PseudoConfig,
    out: &mut String,
) -> Result<(), PseudoError> {
    match sub {
        Subprogram::Function {
            name,
            params,
            return_type,
            body,
        } => {
            let sig = transpile_params(params, cfg)?;
            out.push_str(&format!(
                "{} {}({}) {{\n",
                transpile_type(return_type, cfg),
                name,
                sig
            ));
            for stmt in body {
                transpile_stmt(stmt, out, 1, params)?;
            }
            out.push_str("}\n");
        }
        Subprogram::Action { name, params, body } => {
            let sig = transpile_params(params, cfg)?;
            out.push_str(&format!("void {}({}) {{\n", name, sig));
            for stmt in body {
                transpile_stmt(stmt, out, 1, params)?;
            }
            out.push_str("}\n");
        }
    }
    Ok(())
}

fn transpile_params(params: &[Param], cfg: &PseudoConfig) -> Result<String, PseudoError> {
    let mut parts = Vec::new();
    for p in params {
        let c_type = transpile_type(&p.typ, cfg);
        let part = match p.mode {
            ParamMode::In => format!("{c_type} {}", p.name),
            ParamMode::Out | ParamMode::InOut => format!("{c_type} *{}", p.name),
        };
        parts.push(part);
    }
    Ok(parts.join(", "))
}

fn transpile_decl(
    decl: &Decl,
    cfg: &PseudoConfig,
    out: &mut String,
    indent: usize,
) -> Result<(), PseudoError> {
    match decl {
        Decl::Var { name, typ } => {
            if matches!(typ, TypeName::String) {
                out.push_str(&format!("{}char {}[1024];\n", pad(indent), name));
            } else {
                out.push_str(&format!(
                    "{}{} {};\n",
                    pad(indent),
                    transpile_type(typ, cfg),
                    name
                ));
            }
        }
        Decl::Const { name, typ, value } => {
            out.push_str(&format!(
                "{}const {} {} = {};\n",
                pad(indent),
                transpile_type(typ, cfg),
                name,
                transpile_expr(value)
            ));
        }
    }
    Ok(())
}

fn transpile_stmt(
    stmt: &Stmt,
    out: &mut String,
    indent: usize,
    params: &[Param],
) -> Result<(), PseudoError> {
    match stmt {
        Stmt::Assign { target, expr } => {
            if let Some(param) = params.iter().find(|p| p.name == *target) {
                if matches!(param.mode, ParamMode::Out | ParamMode::InOut) {
                    out.push_str(&format!(
                        "{}*{} = {};\n",
                        pad(indent),
                        target,
                        transpile_expr(expr)
                    ));
                    return Ok(());
                }
            }
            out.push_str(&format!(
                "{}{} = {};\n",
                pad(indent),
                target,
                transpile_expr(expr)
            ));
        }
        Stmt::If {
            cond,
            then_body,
            else_body,
        } => {
            out.push_str(&format!(
                "{}if ({}) {{\n",
                pad(indent),
                transpile_expr(cond)
            ));
            for s in then_body {
                transpile_stmt(s, out, indent + 1, params)?;
            }
            if else_body.is_empty() {
                out.push_str(&format!("{}}}\n", pad(indent)));
            } else {
                out.push_str(&format!("{}}} else {{\n", pad(indent)));
                for s in else_body {
                    transpile_stmt(s, out, indent + 1, params)?;
                }
                out.push_str(&format!("{}}}\n", pad(indent)));
            }
        }
        Stmt::While { cond, body } => {
            out.push_str(&format!(
                "{}while ({}) {{\n",
                pad(indent),
                transpile_expr(cond)
            ));
            for s in body {
                transpile_stmt(s, out, indent + 1, params)?;
            }
            out.push_str(&format!("{}}}\n", pad(indent)));
        }
        Stmt::For {
            var,
            start,
            end,
            step,
            body,
        } => {
            let step_value = step
                .as_ref()
                .map(transpile_expr)
                .unwrap_or_else(|| "1".into());
            out.push_str(&format!(
                "{}for ({} = {}; ({} >= 0 ? {} <= {} : {} >= {}); {} += {}) {{\n",
                pad(indent),
                var,
                transpile_expr(start),
                step_value,
                var,
                transpile_expr(end),
                var,
                transpile_expr(end),
                var,
                step_value
            ));
            for s in body {
                transpile_stmt(s, out, indent + 1, params)?;
            }
            out.push_str(&format!("{}}}\n", pad(indent)));
        }
        Stmt::Return(expr) => {
            out.push_str(&format!(
                "{}return {};\n",
                pad(indent),
                transpile_expr(expr)
            ));
        }
        Stmt::ExprOnly(expr) => {
            out.push_str(&format!("{}{};\n", pad(indent), transpile_expr(expr)));
        }
    }
    Ok(())
}

fn transpile_type(typ: &TypeName, cfg: &PseudoConfig) -> &'static str {
    match typ {
        TypeName::Integer => "int",
        TypeName::Real => cfg.double_type,
        TypeName::Boolean => "bool",
        TypeName::Char => "char",
        TypeName::String => "char",
    }
}

fn transpile_expr(expr: &Expr) -> String {
    match expr {
        Expr::Var(v) => v.clone(),
        Expr::Number(v) => v.clone(),
        Expr::Bool(v) => {
            if *v {
                "true".into()
            } else {
                "false".into()
            }
        }
        Expr::StringLiteral(v) => format!("\"{}\"", v.replace('"', "\\\"")),
        Expr::Unary { op, expr } => {
            let c_op = match op {
                UnaryOp::Not => "!",
                UnaryOp::Neg => "-",
            };
            format!("({}{})", c_op, transpile_expr(expr))
        }
        Expr::Binary { left, op, right } => {
            let c_op = match op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
                BinaryOp::Eq => "==",
                BinaryOp::NotEq => "!=",
                BinaryOp::Lt => "<",
                BinaryOp::Gt => ">",
                BinaryOp::LtEq => "<=",
                BinaryOp::GtEq => ">=",
                BinaryOp::And => "&&",
                BinaryOp::Or => "||",
            };
            format!(
                "({} {} {})",
                transpile_expr(left),
                c_op,
                transpile_expr(right)
            )
        }
        Expr::Call { name, args } => transpile_call(name, args),
    }
}

fn arg_expr(args: &[Expr], idx: usize) -> String {
    args.get(idx)
        .map(transpile_expr)
        .unwrap_or_else(|| "0".into())
}

fn transpile_call(name: &str, args: &[Expr]) -> String {
    let lname = name.to_ascii_lowercase();
    match lname.as_str() {
        "readinteger" => "read_integer()".into(),
        "readreal" => "read_real()".into(),
        "readchar" => "read_char()".into(),
        "readboolean" => "read_boolean()".into(),
        "writeinteger" => format!("printf(\"%d\", {})", arg_expr(args, 0)),
        "writereal" => format!("printf(\"%g\", {})", arg_expr(args, 0)),
        "writechar" => format!("printf(\"%c\", {})", arg_expr(args, 0)),
        "writeboolean" => format!(
            "printf(\"%s\", {} ? \"true\" : \"false\")",
            arg_expr(args, 0)
        ),
        "writestring" => format!("printf(\"%s\", {})", arg_expr(args, 0)),
        _ => {
            let joined = args
                .iter()
                .map(transpile_expr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({})", name, joined)
        }
    }
}

fn pad(indent: usize) -> String {
    "    ".repeat(indent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexer_handles_assign_and_unicode_comparators() {
        let mut lexer = Lexer::new("x := 1\nif x ≤ 3 and x ≠ 2 then y := 4 end if");
        let tokens = lexer.lex().expect("lexer ok");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Assign)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::LessEq)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::NotEq)));
    }

    #[test]
    fn parser_accepts_end_if_and_for_step_minus_one() {
        let code = r#"
            algorithm Demo
                var
                  i: integer;
                end var
                for i := 3 to 1 step -1 do
                  writeInteger(i);
                end for
                if i = 0 then
                  writeInteger(0);
                end if
            end algorithm
            "#;
        let c = pseudo_to_c(code).expect("parse/transpile ok");
        assert!(c.contains("for (i = 3;"));
        assert!(c.contains("i += (-1)"));
        assert!(c.contains("if ((i == 0))"));
    }
}
