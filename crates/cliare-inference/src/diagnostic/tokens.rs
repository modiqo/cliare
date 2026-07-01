use super::model::TokenFeatures;

impl TokenFeatures {
    pub(super) fn from_text(text: &str) -> Self {
        let mut features = Self::default();
        let tokens = tokenize(text).collect::<Vec<_>>();

        for token in &tokens {
            match token_class(token) {
                Some(TokenClass::Identity) => features.identity += 1,
                Some(TokenClass::LocalContext) => features.local_context += 1,
                Some(TokenClass::FixtureInput) => features.fixture_input += 1,
                Some(TokenClass::Network) => features.network += 1,
                Some(TokenClass::RuntimeDependency) => features.runtime_dependency += 1,
                Some(TokenClass::Blocker) => features.blocker += 1,
                None => {}
            }
        }
        features.fixture_input += required_subject_count(&tokens);

        features
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenClass {
    Identity,
    LocalContext,
    FixtureInput,
    Network,
    RuntimeDependency,
    Blocker,
}

fn token_class(token: &str) -> Option<TokenClass> {
    match token {
        "auth" | "authenticate" | "authenticated" | "authentication" | "unauthenticated"
        | "login" | "logged" | "credential" | "credentials" | "token" | "tokens" => {
            Some(TokenClass::Identity)
        }
        "workspace" | "workspaces" | "project" | "projects" | "repository" | "repositories"
        | "repo" | "repos" | "worktree" | "directory" | "directories" | "root" | "cwd"
        | "config" | "configuration" | "git" | "artifact" | "artifacts" | "measurement"
        | "measurements" => Some(TokenClass::LocalContext),
        "argument" | "arguments" | "option" | "options" | "flag" | "flags" | "parameter"
        | "parameters" | "field" | "fields" | "value" | "values" | "input" | "inputs"
        | "operand" | "operands" | "number" | "identifier" | "id" | "name" => {
            Some(TokenClass::FixtureInput)
        }
        "network" | "internet" | "host" | "dns" | "connection" | "connect" | "connecting"
        | "connectivity" | "offline" | "online" | "resolve" | "resolved" | "timeout" | "timed"
        | "rate" | "limit" | "unreachable" => Some(TokenClass::Network),
        "daemon" | "service" | "services" | "container" | "containers" | "socket" | "runtime"
        | "dependency" | "dependencies" | "prerequisite" | "executable" | "binary" | "docker" => {
            Some(TokenClass::RuntimeDependency)
        }
        "required" | "requires" | "require" | "missing" | "not" | "no" | "cannot" | "failed"
        | "failure" | "unavailable" | "refused" | "denied" | "found" | "outside" | "inside"
        | "exceeded" | "set" | "provide" | "provided" | "supply" | "supplied" | "configure"
        | "configured" | "export" => Some(TokenClass::Blocker),
        _ => None,
    }
}

fn required_subject_count(tokens: &[String]) -> usize {
    tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            unknown_required_subject(token)
                && (tokens
                    .get(index + 1)
                    .is_some_and(|next| next == "required" || next == "requires")
                    || tokens
                        .get(index + 1)
                        .is_some_and(|next| next == "is" || next == "are")
                        && tokens.get(index + 2).is_some_and(|next| next == "required"))
        })
        .count()
}

fn unknown_required_subject(token: &str) -> bool {
    token.len() > 1
        && token_class(token).is_none()
        && token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        && !matches!(
            token,
            "the" | "a" | "an" | "this" | "that" | "it" | "when" | "not" | "set"
        )
}

fn tokenize(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter_map(|token| {
            let normalized = token.trim().to_ascii_lowercase();
            if normalized.is_empty() {
                None
            } else {
                Some(normalized)
            }
        })
}
