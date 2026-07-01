#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandPath(Vec<String>);

impl CommandPath {
    pub fn new(path: Vec<String>) -> Self {
        Self(path)
    }

    pub fn as_slice(&self) -> &[String] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn to_vec(&self) -> Vec<String> {
        self.0.clone()
    }
}
