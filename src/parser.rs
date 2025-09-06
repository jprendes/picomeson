use core::fmt;
use hashbrown::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    FormatString(String), // f-string with @0@, @1@ placeholders
    Integer(i64),
    Boolean(bool),
    Array(Vec<Value>),
    Dict(HashMap<String, Value>),
    Identifier(String),
    FunctionCall(String, Vec<Value>, HashMap<String, Value>), // name, args, kwargs
    MethodCall(Box<Value>, String, Vec<Value>, HashMap<String, Value>), // object, method, args, kwargs
    BinaryOp(Box<Value>, BinaryOperator, Box<Value>),
    UnaryOp(UnaryOperator, Box<Value>),
    Subscript(Box<Value>, Box<Value>),
    TernaryOp(Box<Value>, Box<Value>, Box<Value>), // condition ? true_val : false_val
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    In,
    NotIn,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Not,
    Minus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Assignment(String, Value),
    AddAssignment(String, Value),
    Expression(Value),
    If(
        Value,
        Vec<Statement>,
        Vec<(Value, Vec<Statement>)>,
        Option<Vec<Statement>>,
    ), // condition, then, elif_branches, else
    Foreach(String, Value, Vec<Statement>),
    Break,
    Continue,
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    // Literals
    String(String),
    FormatString(String),
    Integer(i64),
    True,
    False,

    // Identifiers and Keywords
    Identifier(String),
    If,
    Elif,
    Else,
    Endif,
    Foreach,
    Endforeach,
    Break,
    Continue,
    And,
    Or,
    Not,
    In,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Assign,
    AddAssign,
    Question,
    Colon,

    // Delimiters
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,

    // Special
    Newline,
    Eof,
}

struct Lexer<'a> {
    chars: core::iter::Peekable<core::str::CharIndices<'a>>,
    current_pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.char_indices().peekable(),
            current_pos: 0,
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, ch)| *ch)
    }

    fn peek_ahead(&mut self, n: usize) -> Option<char> {
        let mut temp = self.chars.clone();
        for _ in 0..n {
            temp.next();
        }
        temp.peek().map(|(_, ch)| *ch)
    }

    fn next_char(&mut self) -> Option<char> {
        if let Some((pos, ch)) = self.chars.next() {
            self.current_pos = pos;
            Some(ch)
        } else {
            None
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.next_char();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        if self.peek_char() == Some('#') {
            self.next_char();
            while let Some(ch) = self.next_char() {
                if ch == '\n' {
                    break;
                }
            }
        }
    }

    fn read_string(&mut self, quote: char) -> String {
        let mut string = String::new();
        let mut escaped = false;

        while let Some(ch) = self.next_char() {
            if escaped {
                // Handle escape sequences
                let escaped_char = match ch {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '\\' => '\\',
                    '\'' => '\'',
                    '"' => '"',
                    '0' => '\0',
                    'a' => '\x07', // Bell
                    'b' => '\x08', // Backspace
                    'f' => '\x0C', // Form feed
                    'v' => '\x0B', // Vertical tab
                    'x' => {
                        // Hexadecimal escape sequence \xHH
                        let mut hex = String::new();
                        for _ in 0..2 {
                            if let Some(hex_char) = self.peek_char() {
                                if hex_char.is_ascii_hexdigit() {
                                    hex.push(self.next_char().unwrap());
                                } else {
                                    break;
                                }
                            }
                        }
                        if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                            byte as char
                        } else {
                            ch // Invalid hex sequence, keep as-is
                        }
                    }
                    'u' => {
                        // Unicode escape sequence \uHHHH
                        let mut hex = String::new();
                        for _ in 0..4 {
                            if let Some(hex_char) = self.peek_char() {
                                if hex_char.is_ascii_hexdigit() {
                                    hex.push(self.next_char().unwrap());
                                } else {
                                    break;
                                }
                            }
                        }
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(unicode_char) = char::from_u32(code) {
                                unicode_char
                            } else {
                                ch
                            }
                        } else {
                            ch
                        }
                    }
                    'U' => {
                        // Unicode escape sequence \UHHHHHHHH
                        let mut hex = String::new();
                        for _ in 0..8 {
                            if let Some(hex_char) = self.peek_char() {
                                if hex_char.is_ascii_hexdigit() {
                                    hex.push(self.next_char().unwrap());
                                } else {
                                    break;
                                }
                            }
                        }
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(unicode_char) = char::from_u32(code) {
                                unicode_char
                            } else {
                                ch
                            }
                        } else {
                            ch
                        }
                    }
                    _ if ch.is_ascii_digit() => {
                        // Octal escape sequence \NNN
                        let mut octal = String::new();
                        octal.push(ch);
                        for _ in 0..2 {
                            if let Some(oct_char) = self.peek_char() {
                                if oct_char.is_digit(8) {
                                    octal.push(self.next_char().unwrap());
                                } else {
                                    break;
                                }
                            }
                        }
                        if let Ok(byte) = u8::from_str_radix(&octal, 8) {
                            byte as char
                        } else {
                            ch
                        }
                    }
                    _ => ch, // Unknown escape, keep the character
                };
                string.push(escaped_char);
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                break;
            } else {
                string.push(ch);
            }
        }

        string
    }

    fn read_multiline_string(&mut self, quote: char) -> String {
        let mut string = String::new();
        let mut consecutive_quotes = 0;

        while let Some(ch) = self.next_char() {
            if ch == quote {
                consecutive_quotes += 1;
                if consecutive_quotes == 3 {
                    consecutive_quotes = 0;
                    // Found closing triple quotes
                    break;
                }
            } else {
                // Add any accumulated quotes that weren't the closing sequence
                for _ in 0..consecutive_quotes {
                    string.push(quote);
                }
                consecutive_quotes = 0;

                // Handle line continuations in multiline strings
                if ch == '\\' && self.peek_char() == Some('\n') {
                    self.next_char(); // consume the newline
                    // Skip leading whitespace on the next line
                    while let Some(ws) = self.peek_char() {
                        if ws == ' ' || ws == '\t' {
                            self.next_char();
                        } else {
                            break;
                        }
                    }
                } else {
                    string.push(ch);
                }
            }
        }

        // Add any remaining quotes (less than 3)
        for _ in 0..consecutive_quotes.min(2) {
            string.push(quote);
        }

        string
    }

    fn read_format_string(&mut self) -> String {
        // f-strings start with f' or f"
        let quote = self.next_char().unwrap();
        self.read_string(quote)
    }

    fn read_identifier(&mut self) -> String {
        let mut ident = String::new();
        while let Some(ch) = self.peek_char() {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(self.next_char().unwrap());
            } else {
                break;
            }
        }
        ident
    }

    fn read_number(&mut self) -> i64 {
        let mut num_str = String::new();

        // Check for hex or octal prefix
        if self.peek_char() == Some('0') {
            num_str.push(self.next_char().unwrap());
            match self.peek_char() {
                Some('x') | Some('X') => {
                    // Hexadecimal
                    self.next_char();
                    let mut hex_str = String::new();
                    while let Some(ch) = self.peek_char() {
                        if ch.is_ascii_hexdigit() || ch == '_' {
                            if ch != '_' {
                                hex_str.push(ch);
                            }
                            self.next_char();
                        } else {
                            break;
                        }
                    }
                    return i64::from_str_radix(&hex_str, 16).unwrap_or(0);
                }
                Some('o') | Some('O') => {
                    // Octal
                    self.next_char();
                    let mut oct_str = String::new();
                    while let Some(ch) = self.peek_char() {
                        if ch.is_digit(8) || ch == '_' {
                            if ch != '_' {
                                oct_str.push(ch);
                            }
                            self.next_char();
                        } else {
                            break;
                        }
                    }
                    return i64::from_str_radix(&oct_str, 8).unwrap_or(0);
                }
                Some('b') | Some('B') => {
                    // Binary
                    self.next_char();
                    let mut bin_str = String::new();
                    while let Some(ch) = self.peek_char() {
                        if ch == '0' || ch == '1' || ch == '_' {
                            if ch != '_' {
                                bin_str.push(ch);
                            }
                            self.next_char();
                        } else {
                            break;
                        }
                    }
                    return i64::from_str_radix(&bin_str, 2).unwrap_or(0);
                }
                _ => {}
            }
        }

        // Decimal number
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() || ch == '_' {
                if ch != '_' {
                    num_str.push(ch);
                }
                self.next_char();
            } else {
                break;
            }
        }

        num_str.parse().unwrap_or(0)
    }

    fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace();

            match self.peek_char() {
                None => {
                    tokens.push(Token::Eof);
                    break;
                }
                Some('\n') => {
                    self.next_char();
                    // Skip consecutive newlines
                    while self.peek_char() == Some('\n') {
                        self.next_char();
                    }
                    tokens.push(Token::Newline);
                }
                Some('#') => {
                    self.skip_comment();
                    // Comments implicitly end the line
                    tokens.push(Token::Newline);
                }
                Some('\'') | Some('"') => {
                    let quote = self.peek_char().unwrap();
                    // Check for triple quotes (multiline strings)
                    if self.peek_ahead(1) == Some(quote) && self.peek_ahead(2) == Some(quote) {
                        self.next_char(); // First quote
                        self.next_char(); // Second quote
                        self.next_char(); // Third quote
                        let string = self.read_multiline_string(quote);
                        tokens.push(Token::String(string));
                    } else {
                        self.next_char();
                        let string = self.read_string(quote);
                        tokens.push(Token::String(string));
                    }
                }
                Some('f') if matches!(self.peek_ahead(1), Some('\'') | Some('"')) => {
                    // f-string
                    self.next_char(); // consume 'f'
                    let string = self.read_format_string();
                    tokens.push(Token::FormatString(string));
                }
                Some('r') if matches!(self.peek_ahead(1), Some('\'') | Some('"')) => {
                    // raw string (treat like regular string but without escape processing)
                    self.next_char(); // consume 'r'
                    let quote = self.next_char().unwrap();
                    let mut string = String::new();
                    while let Some(ch) = self.next_char() {
                        if ch == quote {
                            break;
                        }
                        string.push(ch);
                    }
                    tokens.push(Token::String(string));
                }
                Some('0'..='9') => {
                    let num = self.read_number();
                    tokens.push(Token::Integer(num));
                }
                Some('a'..='z') | Some('A'..='Z') | Some('_') => {
                    let ident = self.read_identifier();
                    let token = match ident.as_str() {
                        "true" => Token::True,
                        "false" => Token::False,
                        "if" => Token::If,
                        "elif" => Token::Elif,
                        "else" => Token::Else,
                        "endif" => Token::Endif,
                        "foreach" => Token::Foreach,
                        "endforeach" => Token::Endforeach,
                        "break" => Token::Break,
                        "continue" => Token::Continue,
                        "and" => Token::And,
                        "or" => Token::Or,
                        "not" => Token::Not,
                        "in" => Token::In,
                        _ => Token::Identifier(ident),
                    };
                    tokens.push(token);
                }
                Some('+') => {
                    self.next_char();
                    if self.peek_char() == Some('=') {
                        self.next_char();
                        tokens.push(Token::AddAssign);
                    } else {
                        tokens.push(Token::Plus);
                    }
                }
                Some('-') => {
                    self.next_char();
                    tokens.push(Token::Minus);
                }
                Some('*') => {
                    self.next_char();
                    tokens.push(Token::Star);
                }
                Some('/') => {
                    self.next_char();
                    // Check if it's a division operator or path separator in context
                    tokens.push(Token::Slash);
                }
                Some('%') => {
                    self.next_char();
                    tokens.push(Token::Percent);
                }
                Some('=') => {
                    self.next_char();
                    if self.peek_char() == Some('=') {
                        self.next_char();
                        tokens.push(Token::Eq);
                    } else {
                        tokens.push(Token::Assign);
                    }
                }
                Some('!') => {
                    self.next_char();
                    if self.peek_char() == Some('=') {
                        self.next_char();
                        tokens.push(Token::Ne);
                    }
                    // Note: standalone '!' is not a valid token in Meson
                }
                Some('<') => {
                    self.next_char();
                    if self.peek_char() == Some('=') {
                        self.next_char();
                        tokens.push(Token::Le);
                    } else {
                        tokens.push(Token::Lt);
                    }
                }
                Some('>') => {
                    self.next_char();
                    if self.peek_char() == Some('=') {
                        self.next_char();
                        tokens.push(Token::Ge);
                    } else {
                        tokens.push(Token::Gt);
                    }
                }
                Some('?') => {
                    self.next_char();
                    tokens.push(Token::Question);
                }
                Some(':') => {
                    self.next_char();
                    tokens.push(Token::Colon);
                }
                Some('(') => {
                    self.next_char();
                    tokens.push(Token::LeftParen);
                }
                Some(')') => {
                    self.next_char();
                    tokens.push(Token::RightParen);
                }
                Some('[') => {
                    self.next_char();
                    tokens.push(Token::LeftBracket);
                }
                Some(']') => {
                    self.next_char();
                    tokens.push(Token::RightBracket);
                }
                Some('{') => {
                    self.next_char();
                    tokens.push(Token::LeftBrace);
                }
                Some('}') => {
                    self.next_char();
                    tokens.push(Token::RightBrace);
                }
                Some(',') => {
                    self.next_char();
                    tokens.push(Token::Comma);
                }
                Some('.') => {
                    self.next_char();
                    tokens.push(Token::Dot);
                }
                _ => {
                    self.next_char(); // Skip unknown characters
                }
            }
        }

        tokens
    }
}

// ... (Parser implementation remains mostly the same, with these key additions)

impl Parser {
    pub fn new(input: &str) -> Self {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        Parser { tokens, pos: 0 }
    }

    // ... (rest of the parser methods remain the same as before)

    fn dict_elements(&mut self) -> Result<HashMap<String, Value>, ParseError> {
        let mut dict = HashMap::new();

        if matches!(self.peek(), Token::RightBrace) {
            return Ok(dict);
        }

        loop {
            // Meson allows both string keys and identifier keys in dict literals
            let key = match self.peek() {
                Token::String(s) => {
                    let key = s.clone();
                    self.advance();
                    key
                }
                Token::Identifier(s) => {
                    // Support for shorthand key notation (identifier as key)
                    let key = s.clone();
                    self.advance();
                    key
                }
                Token::FormatString(s) => {
                    let key = s.clone();
                    self.advance();
                    key
                }
                _ => return Err(ParseError::UnexpectedToken),
            };

            self.expect(&Token::Colon)?;
            let value = self.expression()?;
            dict.insert(key, value);

            if !self.match_token(&Token::Comma) {
                break;
            }

            // Allow trailing comma
            if matches!(self.peek(), Token::RightBrace) {
                break;
            }
        }

        Ok(dict)
    }

    // Allow for keyword arguments without parentheses in some contexts
    fn arguments(&mut self) -> Result<(Vec<Value>, HashMap<String, Value>), ParseError> {
        let mut args = Vec::new();
        let mut kwargs = HashMap::new();
        let mut seen_kwarg = false;

        if matches!(self.peek(), Token::RightParen) {
            return Ok((args, kwargs));
        }

        loop {
            // Check for keyword argument
            if let Token::Identifier(name) = self.peek() {
                let saved_pos = self.pos;
                let name_clone = name.clone();
                self.advance();
                if self.match_token(&Token::Colon) {
                    // It's a keyword argument
                    seen_kwarg = true;
                    let value = self.expression()?;
                    kwargs.insert(name_clone, value);
                } else {
                    // It's a positional argument (but only if we haven't seen kwargs yet)
                    if seen_kwarg {
                        return Err(ParseError::UnexpectedToken); // Can't have positional after keyword
                    }
                    self.pos = saved_pos;
                    args.push(self.expression()?);
                }
            } else {
                if seen_kwarg {
                    return Err(ParseError::UnexpectedToken); // Can't have positional after keyword
                }
                args.push(self.expression()?);
            }

            if !self.match_token(&Token::Comma) {
                break;
            }

            // Allow trailing comma
            if matches!(self.peek(), Token::RightParen) {
                break;
            }
        }

        Ok((args, kwargs))
    }

    pub fn parse(&mut self) -> Result<Vec<Statement>, ParseError> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            statements.push(self.statement()?);
        }

        Ok(statements)
    }

    fn statement(&mut self) -> Result<Statement, ParseError> {
        match &self.peek() {
            Token::If => self.if_statement(),
            Token::Foreach => self.foreach_statement(),
            Token::Break => {
                self.advance();
                self.expect_newline_or_eof()?;
                Ok(Statement::Break)
            }
            Token::Continue => {
                self.advance();
                self.expect_newline_or_eof()?;
                Ok(Statement::Continue)
            }
            _ => {
                // Try assignment or expression
                let expr = self.expression()?;

                // Check for assignment
                if let Value::Identifier(name) = &expr {
                    if self.match_token(&Token::Assign) {
                        let value = self.expression()?;
                        self.expect_newline_or_eof()?;
                        return Ok(Statement::Assignment(name.clone(), value));
                    } else if self.match_token(&Token::AddAssign) {
                        let value = self.expression()?;
                        self.expect_newline_or_eof()?;
                        return Ok(Statement::AddAssignment(name.clone(), value));
                    }
                }

                self.expect_newline_or_eof()?;
                Ok(Statement::Expression(expr))
            }
        }
    }

    fn if_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(&Token::If)?;
        let condition = self.expression()?;
        self.expect_newline()?;

        let mut then_branch = Vec::new();
        while !matches!(self.peek(), Token::Elif | Token::Else | Token::Endif) {
            then_branch.push(self.statement()?);
        }

        let mut elif_branches = Vec::new();
        while self.match_token(&Token::Elif) {
            let elif_condition = self.expression()?;
            self.expect_newline()?;

            let mut elif_body = Vec::new();
            while !matches!(self.peek(), Token::Elif | Token::Else | Token::Endif) {
                elif_body.push(self.statement()?);
            }
            elif_branches.push((elif_condition, elif_body));
        }

        let else_branch = if self.match_token(&Token::Else) {
            self.expect_newline()?;
            let mut else_body = Vec::new();
            while !matches!(self.peek(), Token::Endif) {
                else_body.push(self.statement()?);
            }
            Some(else_body)
        } else {
            None
        };

        self.expect(&Token::Endif)?;
        Ok(Statement::If(
            condition,
            then_branch,
            elif_branches,
            else_branch,
        ))
    }

    fn foreach_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(&Token::Foreach)?;
        let var = if let Token::Identifier(name) = self.advance() {
            name
        } else {
            return Err(ParseError::UnexpectedToken);
        };
        self.expect(&Token::Colon)?;
        let iterable = self.expression()?;
        self.expect_newline()?;

        let mut body = Vec::new();
        while !matches!(self.peek(), Token::Endforeach) {
            body.push(self.statement()?);
        }

        self.expect(&Token::Endforeach)?;
        Ok(Statement::Foreach(var, iterable, body))
    }

    fn expression(&mut self) -> Result<Value, ParseError> {
        self.ternary()
    }

    fn ternary(&mut self) -> Result<Value, ParseError> {
        let expr = self.logical_or()?;

        if self.match_token(&Token::Question) {
            let true_val = self.expression()?;
            self.expect(&Token::Colon)?;
            let false_val = self.expression()?;
            return Ok(Value::TernaryOp(
                Box::new(expr),
                Box::new(true_val),
                Box::new(false_val),
            ));
        }

        Ok(expr)
    }

    fn logical_or(&mut self) -> Result<Value, ParseError> {
        let mut left = self.logical_and()?;

        while self.match_token(&Token::Or) {
            let right = self.logical_and()?;
            left = Value::BinaryOp(Box::new(left), BinaryOperator::Or, Box::new(right));
        }

        Ok(left)
    }

    fn logical_and(&mut self) -> Result<Value, ParseError> {
        let mut left = self.in_expr()?;

        while self.match_token(&Token::And) {
            let right = self.in_expr()?;
            left = Value::BinaryOp(Box::new(left), BinaryOperator::And, Box::new(right));
        }

        Ok(left)
    }

    fn in_expr(&mut self) -> Result<Value, ParseError> {
        let mut left = self.equality()?;

        if self.match_token(&Token::In) {
            let right = self.equality()?;
            left = Value::BinaryOp(Box::new(left), BinaryOperator::In, Box::new(right));
        } else if self.match_token(&Token::Not) && self.match_token(&Token::In) {
            let right = self.equality()?;
            left = Value::BinaryOp(Box::new(left), BinaryOperator::NotIn, Box::new(right));
        }

        Ok(left)
    }

    fn equality(&mut self) -> Result<Value, ParseError> {
        let mut left = self.comparison()?;

        while let Some(op) = self.match_tokens(&[Token::Eq, Token::Ne]) {
            let right = self.comparison()?;
            let op = match op {
                Token::Eq => BinaryOperator::Eq,
                Token::Ne => BinaryOperator::Ne,
                _ => unreachable!(),
            };
            left = Value::BinaryOp(Box::new(left), op, Box::new(right));
        }

        Ok(left)
    }

    fn comparison(&mut self) -> Result<Value, ParseError> {
        let mut left = self.addition()?;

        while let Some(op) = self.match_tokens(&[Token::Lt, Token::Le, Token::Gt, Token::Ge]) {
            let right = self.addition()?;
            let op = match op {
                Token::Lt => BinaryOperator::Lt,
                Token::Le => BinaryOperator::Le,
                Token::Gt => BinaryOperator::Gt,
                Token::Ge => BinaryOperator::Ge,
                _ => unreachable!(),
            };
            left = Value::BinaryOp(Box::new(left), op, Box::new(right));
        }

        Ok(left)
    }

    fn addition(&mut self) -> Result<Value, ParseError> {
        let mut left = self.multiplication()?;

        while let Some(op) = self.match_tokens(&[Token::Plus, Token::Minus]) {
            let right = self.multiplication()?;
            let op = match op {
                Token::Plus => BinaryOperator::Add,
                Token::Minus => BinaryOperator::Sub,
                _ => unreachable!(),
            };
            left = Value::BinaryOp(Box::new(left), op, Box::new(right));
        }

        Ok(left)
    }

    fn multiplication(&mut self) -> Result<Value, ParseError> {
        let mut left = self.unary()?;

        while let Some(op) = self.match_tokens(&[Token::Star, Token::Slash, Token::Percent]) {
            let right = self.unary()?;
            let op = match op {
                Token::Star => BinaryOperator::Mul,
                Token::Slash => BinaryOperator::Div,
                Token::Percent => BinaryOperator::Mod,
                _ => unreachable!(),
            };
            left = Value::BinaryOp(Box::new(left), op, Box::new(right));
        }

        Ok(left)
    }

    fn unary(&mut self) -> Result<Value, ParseError> {
        if self.match_token(&Token::Not) {
            let expr = self.unary()?;
            return Ok(Value::UnaryOp(UnaryOperator::Not, Box::new(expr)));
        }

        if self.match_token(&Token::Minus) {
            let expr = self.unary()?;
            return Ok(Value::UnaryOp(UnaryOperator::Minus, Box::new(expr)));
        }

        self.postfix()
    }

    fn postfix(&mut self) -> Result<Value, ParseError> {
        let mut expr = self.primary()?;

        loop {
            match self.peek() {
                Token::LeftParen => {
                    // Function or method call
                    self.advance();
                    let (args, kwargs) = self.arguments()?;
                    self.expect(&Token::RightParen)?;

                    if let Value::Identifier(name) = expr {
                        expr = Value::FunctionCall(name, args, kwargs);
                    } else if let Value::MethodCall(obj, method, _, _) = expr {
                        expr = Value::MethodCall(obj, method, args, kwargs);
                    } else {
                        return Err(ParseError::UnexpectedToken);
                    }
                }
                Token::LeftBracket => {
                    // Subscript
                    self.advance();
                    let index = self.expression()?;
                    self.expect(&Token::RightBracket)?;
                    expr = Value::Subscript(Box::new(expr), Box::new(index));
                }
                Token::Dot => {
                    // Method call
                    self.advance();
                    if let Token::Identifier(method) = self.advance() {
                        expr = Value::MethodCall(Box::new(expr), method, vec![], HashMap::new());
                    } else {
                        return Err(ParseError::UnexpectedToken);
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn primary(&mut self) -> Result<Value, ParseError> {
        match self.advance() {
            Token::String(s) => Ok(Value::String(s)),
            Token::FormatString(s) => Ok(Value::FormatString(s)),
            Token::Integer(i) => Ok(Value::Integer(i)),
            Token::True => Ok(Value::Boolean(true)),
            Token::False => Ok(Value::Boolean(false)),
            Token::Identifier(name) => Ok(Value::Identifier(name)),
            Token::LeftParen => {
                let expr = self.expression()?;
                self.expect(&Token::RightParen)?;
                Ok(expr)
            }
            Token::LeftBracket => {
                let elements = self.array_elements()?;
                self.expect(&Token::RightBracket)?;
                Ok(Value::Array(elements))
            }
            Token::LeftBrace => {
                let dict = self.dict_elements()?;
                self.expect(&Token::RightBrace)?;
                Ok(Value::Dict(dict))
            }
            _ => Err(ParseError::UnexpectedToken),
        }
    }

    fn array_elements(&mut self) -> Result<Vec<Value>, ParseError> {
        let mut elements = Vec::new();

        if matches!(self.peek(), Token::RightBracket) {
            return Ok(elements);
        }

        loop {
            elements.push(self.expression()?);
            if !self.match_token(&Token::Comma) {
                break;
            }
            // Allow trailing comma
            if matches!(self.peek(), Token::RightBracket) {
                break;
            }
        }

        Ok(elements)
    }

    // Helper methods
    fn peek_with_newline(&self) -> Token {
        self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof)
    }

    fn peek(&self) -> Token {
        self.tokens[self.pos..]
            .iter()
            .find(|&t| t != &Token::Newline)
            .cloned()
            .unwrap_or(Token::Eof)
    }

    fn skip_newline(&mut self) {
        while self.peek_with_newline() == Token::Newline {
            self.pos += 1;
        }
    }

    fn advance(&mut self) -> Token {
        while self.peek_with_newline() == Token::Newline {
            self.pos += 1;
        }
        let token = self.peek();
        if !self.is_at_end() {
            self.pos += 1;
        }
        token
    }

    fn match_token(&mut self, token: &Token) -> bool {
        if core::mem::discriminant(&self.peek()) == core::mem::discriminant(token) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_tokens(&mut self, tokens: &[Token]) -> Option<Token> {
        for token in tokens {
            if core::mem::discriminant(&self.peek()) == core::mem::discriminant(token) {
                return Some(self.advance());
            }
        }
        None
    }

    fn expect(&mut self, token: &Token) -> Result<(), ParseError> {
        if core::mem::discriminant(&self.peek()) == core::mem::discriminant(token) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken)
        }
    }

    fn expect_newline(&mut self) -> Result<(), ParseError> {
        if matches!(self.peek_with_newline(), Token::Newline | Token::Eof) {
            self.skip_newline();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken)
        }
    }

    fn expect_newline_or_eof(&mut self) -> Result<(), ParseError> {
        if matches!(self.peek_with_newline(), Token::Newline | Token::Eof) {
            self.skip_newline();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken)
        }
    }

    fn is_at_end(&mut self) -> bool {
        matches!(self.peek(), Token::Eof)
    }
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken => write!(f, "Unexpected token"),
        }
    }
}

impl core::error::Error for ParseError {}

// Example usage
pub fn parse_meson_file(content: &str) -> Result<Vec<Statement>, ParseError> {
    let mut parser = Parser::new(content);
    match parser.parse() {
        Ok(statements) => Ok(statements),
        Err(e) => {
            println!("Parse error: {}", e);
            println!("Tokens: {:?}", &parser.tokens[..parser.pos]);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dict_with_comments() {
        let input = r#"
cpu_family_aliases = {
    # aarch64
    'arm64' : 'aarch64',
    # cris
    'crisv32' : 'cris',
}
"#;
        let result = parse_meson_file(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_format_string() {
        let input = r#"
_defsym = '-Wl,--defsym=@0@=@1@main'.format(start, global_prefix)
"#;
        let result = parse_meson_file(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiline_string() {
        let input = r#"
message = '''
Unsupported architecture: "@0@"

    Read the Supported Architectures section in README.md
    to learn how to add a new architecture.
'''
"#;
        let result = parse_meson_file(input);
        assert!(result.is_ok());
    }
}
