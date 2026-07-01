use std::collections::BTreeSet;

#[derive(Debug)]
pub(super) struct TokenSet {
    pub(super) tokens: BTreeSet<String>,
}

impl TokenSet {
    pub(super) fn from_text(text: &str) -> Self {
        let mut tokens = BTreeSet::new();
        for token in tokenize(text) {
            tokens.insert(token);
        }
        Self { tokens }
    }

    pub(super) fn contains(&self, token: &str) -> bool {
        let needle_variants = token_variants(token);
        needle_variants
            .iter()
            .any(|variant| self.tokens.contains(variant))
            || self.tokens.iter().any(|own_token| {
                token_variants(own_token)
                    .iter()
                    .any(|variant| variant == token)
            })
    }

    pub(super) fn intersects(&self, other: &Self) -> bool {
        self.tokens.iter().any(|token| other.contains(token))
    }

    pub(super) fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}

pub(super) fn normalize_command_path(raw: &[String]) -> Vec<String> {
    if raw.len() == 1 {
        raw[0].split_whitespace().map(ToOwned::to_owned).collect()
    } else {
        raw.to_vec()
    }
}

pub(super) fn normalize_phrase(text: &str) -> String {
    tokenize(text).join(" ")
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            current.push(ch.to_ascii_lowercase());
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn token_variants(token: &str) -> Vec<String> {
    let mut variants = vec![token.to_owned()];
    if token.len() > 3 && token.ends_with('s') && !token.ends_with("ss") && !token.ends_with("us") {
        variants.push(token.trim_end_matches('s').to_owned());
    }
    variants
}
