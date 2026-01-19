use crate::diagnostics::{Diagnostic, DiagnosticLevel, IntoDiagnostic};
use miette::Diagnostic as MietteDiagnostic;
use thiserror::Error;

#[derive(Error, Debug, MietteDiagnostic)]
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

impl IntoDiagnostic for CompileError {
    fn into_diagnostic(self) -> Vec<Diagnostic> {
        match self {
            CompileError::ParseError { span, message, .. } => vec![Diagnostic {
                level: DiagnosticLevel::Error,
                message,
                span,
                code: Some("parse_error".to_string()),
                ..Default::default()
            }],
            CompileError::ValidationError { span, message, .. } => vec![Diagnostic {
                level: DiagnosticLevel::Error,
                message,
                span,
                code: Some("validation_error".to_string()),
                ..Default::default()
            }],
            CompileError::KdlError(e) => {
                use miette::Diagnostic as MietteDiagnostic;

                let span = e
                    .labels()
                    .and_then(|mut labels| labels.next())
                    .map(|l| *l.inner())
                    .unwrap_or_else(|| (0, 0).into());

                let message = e.to_string();

                vec![Diagnostic {
                    level: DiagnosticLevel::Error,
                    message,
                    span,
                    code: Some("kdl_error".to_string()),
                    hints: e.help().map(|h| vec![h.to_string()]).unwrap_or_default(),
                    ..Default::default()
                }]
            }
        }
    }
}
