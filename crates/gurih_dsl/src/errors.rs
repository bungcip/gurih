use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum CompileError {
    #[error("Parse error")]
    #[diagnostic(code(gurih::parse_error))]
    ParseError {
        #[source_code]
        src: String,
        #[label("here")]
        span: miette::SourceSpan,
        message: String,
    },

    #[error("Validation error: {message}")]
    #[diagnostic(code(gurih::validation_error))]
    ValidationError {
        #[source_code]
        src: String,
        #[label("here")]
        span: miette::SourceSpan,
        message: String,
    },
    
    #[error("KDL error: {0}")]
    #[diagnostic(code(gurih::kdl_error))]
    KdlError(#[from] kdl::KdlError),
}
