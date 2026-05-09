use skymemory_core::SkyError;

#[derive(Debug, Clone)]
pub enum QueryNode {
    VectorSimilar {
        terms: String,
    },
    PropertyFilter {
        key: String,
        op: FilterOp,
        value: String,
    },
    BoostRecent {
        weight: f32,
    },
    Expand {
        depth: usize,
    },
    And(Box<QueryNode>, Box<QueryNode>),
    Or(Box<QueryNode>, Box<QueryNode>),
}

#[derive(Debug, Clone)]
pub enum FilterOp {
    Eq,
    Neq,
    Gt,
    Lt,
    Gte,
    Lte,
}

pub fn parse_query(input: &str) -> Result<QueryNode, SkyError> {
    let tokens = tokenize(input)?;
    parse_expr(&tokens, &mut 0)
}

#[derive(Debug, Clone)]
enum Token {
    Ident(String),
    StringLit(String),
    Number(String),
    LParen,
    RParen,
    Eq,
    And,
    Or,
    Boost,
    Expand,
    Similar,
}

fn tokenize(input: &str) -> Result<Vec<Token>, SkyError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        if chars[i] == '(' {
            tokens.push(Token::LParen);
            i += 1;
            continue;
        }
        if chars[i] == ')' {
            tokens.push(Token::RParen);
            i += 1;
            continue;
        }
        if chars[i] == '=' {
            tokens.push(Token::Eq);
            i += 1;
            continue;
        }
        if chars[i] == '"' {
            i += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != '"' {
                s.push(chars[i]);
                i += 1;
            }
            if i < chars.len() { i += 1; }
            tokens.push(Token::StringLit(s));
            continue;
        }
        if chars[i].is_ascii_digit() || (chars[i] == '-' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()) {
            let mut s = String::new();
            s.push(chars[i]);
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                s.push(chars[i]);
                i += 1;
            }
            tokens.push(Token::Number(s));
            continue;
        }
        if chars[i].is_ascii_alphabetic() || chars[i] == '_' {
            let mut s = String::new();
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '-' || chars[i] == '.') {
                s.push(chars[i]);
                i += 1;
            }
            match s.as_str() {
                "AND" | "and" => tokens.push(Token::And),
                "OR" | "or" => tokens.push(Token::Or),
                "BOOST" | "boost" => tokens.push(Token::Boost),
                "EXPAND" | "expand" => tokens.push(Token::Expand),
                "VECTOR_SIMILAR" | "vector_similar" => tokens.push(Token::Similar),
                _ => tokens.push(Token::Ident(s)),
            }
            continue;
        }
        i += 1;
    }
    Ok(tokens)
}

fn parse_expr(tokens: &[Token], pos: &mut usize) -> Result<QueryNode, SkyError> {
    let mut left = parse_primary(tokens, pos)?;
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::And => {
                *pos += 1;
                let right = parse_primary(tokens, pos)?;
                left = QueryNode::And(Box::new(left), Box::new(right));
            }
            Token::Or => {
                *pos += 1;
                let right = parse_primary(tokens, pos)?;
                left = QueryNode::Or(Box::new(left), Box::new(right));
            }
            _ => break,
        }
    }
    Ok(left)
}

fn parse_primary(tokens: &[Token], pos: &mut usize) -> Result<QueryNode, SkyError> {
    if *pos >= tokens.len() {
        return Err(SkyError::QueryParse("unexpected end of query".into()));
    }
    match &tokens[*pos] {
        Token::LParen => {
            *pos += 1;
            let node = parse_expr(tokens, pos)?;
            if *pos < tokens.len() && matches!(tokens[*pos], Token::RParen) {
                *pos += 1;
            }
            Ok(node)
        }
        Token::Similar => {
            *pos += 1;
            let terms = if *pos < tokens.len() {
                match &tokens[*pos] {
                    Token::StringLit(s) => { *pos += 1; s.clone() }
                    Token::LParen => {
                        *pos += 1;
                        let s = if let Token::StringLit(s) = &tokens[*pos] {
                            s.clone()
                        } else {
                            return Err(SkyError::QueryParse("expected string after VECTOR_SIMILAR(".into()));
                        };
                        *pos += 1;
                        if *pos < tokens.len() && matches!(tokens[*pos], Token::RParen) {
                            *pos += 1;
                        }
                        s
                    }
                    _ => return Err(SkyError::QueryParse("expected string after VECTOR_SIMILAR".into())),
                }
            } else {
                return Err(SkyError::QueryParse("expected string after VECTOR_SIMILAR".into()));
            };
            Ok(QueryNode::VectorSimilar { terms })
        }
        Token::Boost => {
            *pos += 1;
            let mut weight = 1.5;
            if *pos < tokens.len() {
                if let Token::Ident(s) = &tokens[*pos] {
                    if s == "recent" { *pos += 1; }
                }
                if *pos < tokens.len() {
                    if let Token::Number(n) = &tokens[*pos] {
                        weight = n.parse().unwrap_or(1.5);
                        *pos += 1;
                    }
                }
            }
            Ok(QueryNode::BoostRecent { weight })
        }
        Token::Expand => {
            *pos += 1;
            let mut depth = 2;
            if *pos < tokens.len() {
                if let Token::Ident(s) = &tokens[*pos] {
                    if s.starts_with("depth=") {
                        depth = s.strip_prefix("depth=").and_then(|v| v.parse().ok()).unwrap_or(2);
                        *pos += 1;
                    }
                }
            }
            Ok(QueryNode::Expand { depth })
        }
        Token::Ident(key) => {
            let key = key.clone();
            *pos += 1;
            if *pos < tokens.len() && matches!(tokens[*pos], Token::Eq) {
                *pos += 1;
                let value = if *pos < tokens.len() {
                    match &tokens[*pos] {
                        Token::StringLit(s) => { *pos += 1; s.clone() }
                        Token::Ident(s) => { *pos += 1; s.clone() }
                        Token::Number(s) => { *pos += 1; s.clone() }
                        _ => return Err(SkyError::QueryParse("expected value after =".into())),
                    }
                } else {
                    return Err(SkyError::QueryParse("expected value after =".into()));
                };
                Ok(QueryNode::PropertyFilter {
                    key,
                    op: FilterOp::Eq,
                    value,
                })
            } else {
                Err(SkyError::QueryParse(format!("unexpected token after '{}'", key)))
            }
        }
        _ => Err(SkyError::QueryParse(format!("unexpected token at position {}", *pos))),
    }
}
