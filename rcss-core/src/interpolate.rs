use std::borrow::Cow;

/// Interpolation token.
/// Different tokens are handled differently:
/// - UnwrapLiteral: just place original string as is, into source css file.
/// Later can be extended with compile time or runtime expressions.

pub enum Token {
    UnwrapLiteral(String),
}

/// Token with information about its position in source.
pub struct TokenWithId {
    pub token: Token,
    id: String,
}

// Store each token in array, and save
pub struct Interpolation {
    pub tokens: Vec<TokenWithId>,
}
impl Interpolation {
    pub fn unwrap_literals(&self, source: &str) -> String {
        let mut result = String::from(source);
        for token in &self.tokens {
            match &token.token {
                Token::UnwrapLiteral(s) => result = result.replace(&token.id, s),
            }
        }
        result
    }
}

/// Find any occurrences of ${} in source string and replace it with TOKEN_ID.
/// Returns: Interpolation object with info about all tokens that was replaced
/// And source string with replaced tokens.
pub fn handle_interpolate(mut source: &str) -> (Interpolation, Cow<'_, str>) {
    let mut state = Interpolation { tokens: vec![] };

    let mut result = String::new();
    let mut last_id = 0;
    while let Some(start) = source[..].find("${") {
        let end = source[start..].find("}").unwrap_or_else(|| {
            panic!(
                "No closing bracket found for interpolation token at position {}",
                start
            )
        }) + start;

        let token = &source[start + 2..end].trim_start();
        let token_id = format!("__RCSS__TOKEN_{}", last_id);
        last_id += 1;

        if token.starts_with("\"") {
            let token = token.trim_matches(|c: char| c.is_whitespace() || c == '"');
            state.tokens.push(TokenWithId {
                token: Token::UnwrapLiteral(token.to_string()),
                id: token_id.clone(),
            });
        } else {
            panic!(
                "Only string literals are supported in interpolation \"#{{..}}\", got=\"{}\"",
                token
            );
        }

        result.push_str(&source[..start]);
        result.push_str(&token_id);
        source = &source[end + 1..];
    }
    if state.tokens.is_empty() {
        return (state, Cow::Borrowed(source));
    }
    result.push_str(&source);
    (state, result.into())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_handle_interpolate() {
        let source = "background-color: red; left: ${\"0em\"};";
        let (interpolation, result) = handle_interpolate(source);
        assert_eq!(interpolation.tokens.len(), 1);
        assert_eq!(result, "background-color: red; left: __RCSS__TOKEN_0;");
        match &interpolation.tokens[0].token {
            Token::UnwrapLiteral(s) => assert_eq!(s, "0em"),
        }
    }

    #[test]
    fn test_handle_unwrap() {
        let source = "background-color: red; left: ${\"0em\"}; color: #${\"ff0000\"};";
        let (interpolation, result) = handle_interpolate(source);
        assert_eq!(
            result,
            "background-color: red; left: __RCSS__TOKEN_0; color: #__RCSS__TOKEN_1;"
        );
        assert_eq!(interpolation.tokens.len(), 2);
        let result = interpolation.unwrap_literals(&result);
        assert_eq!(result, "background-color: red; left: 0em; color: #ff0000;");
    }
}
