use std::fmt::{self, Write as _};

#[derive(Debug, Default)]
pub(crate) struct MarkdownBuffer {
    text: String,
}

impl MarkdownBuffer {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn line(&mut self, args: fmt::Arguments<'_>) {
        self.text
            .write_fmt(args)
            .expect("writing to string cannot fail");
        self.text.push('\n');
    }

    pub(crate) fn blank_line(&mut self) {
        self.text.push('\n');
    }

    pub(crate) fn into_string(self) -> String {
        self.text
    }
}

#[cfg(test)]
mod tests {
    use super::MarkdownBuffer;

    #[test]
    fn writes_formatted_lines_and_blank_lines() {
        let mut buffer = MarkdownBuffer::new();
        buffer.line(format_args!("# {}", "Title"));
        buffer.blank_line();
        buffer.line(format_args!("score: {:.0}", 91.2));

        assert_eq!(buffer.into_string(), "# Title\n\nscore: 91\n");
    }
}
