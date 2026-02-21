use crate::diagnostics::{Diagnostic, DiagnosticLevel, IntoDiagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompileError {
    #[error("Parse error")]
    ParseError { span: SourceSpan, message: String },

    #[error("Validation error: {message}")]
    ValidationError { span: SourceSpan, message: String },

    #[error("KDL error: {0}")]
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
                    span: span.into(),
                    code: Some("kdl_error".to_string()),
                    hints: e.help().map(|h| vec![h.to_string()]).unwrap_or_default(),
                    ..Default::default()
                }]
            }
        }
    }
}
