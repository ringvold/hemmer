mod error;
mod pipeline;
mod transformers;

pub use error::Error;
pub use pipeline::{Pipeline, PipelineResult};
pub use transformers::inline_css::InlineCssConfig;
pub use transformers::tailwind::email_config as tailwind_email_config;
