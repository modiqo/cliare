use clap::{Args, ValueEnum};

#[derive(Debug, Args)]
#[command(disable_help_flag = true)]
pub struct MetadataArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = MetadataFormat::Text)]
    pub format: MetadataFormat,

    /// Print help. With --format json, emit a parseable metadata contract.
    #[arg(long)]
    pub help: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum MetadataFormat {
    Text,
    Json,
}

impl MetadataFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
        }
    }
}
