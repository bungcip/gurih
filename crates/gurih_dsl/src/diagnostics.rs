use annotate_snippets::{AnnotationKind, Level, Renderer, Snippet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourceSpan {
    offset: usize,
    length: usize,
}

impl SourceSpan {
    pub fn new(offset: usize, length: usize) -> Self {
        Self { offset, length }
    }
    pub fn offset(&self) -> usize {
        self.offset
    }
    pub fn len(&self) -> usize {
        self.length
    }
}

impl From<miette::SourceSpan> for SourceSpan {
    fn from(span: miette::SourceSpan) -> Self {
        Self::new(span.offset(), span.len())
    }
}

impl From<(usize, usize)> for SourceSpan {
    fn from(value: (usize, usize)) -> Self {
        Self::new(value.0, value.1)
    }
}

/// Diagnostic severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagnosticLevel {
    #[default]
    Error,
    Warning,
    Note,
}

/// Individual diagnostic with rich context
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub span: SourceSpan,
    pub code: Option<String>,               // Error code like "E001"
    pub hints: Vec<String>,                 // Suggestions for fixing
    pub related: Vec<(SourceSpan, String)>, // Related locations with message
}

impl Default for Diagnostic {
    fn default() -> Self {
        Self {
            level: DiagnosticLevel::default(),
            message: String::new(),
            span: (0, 0).into(),
            code: None,
            hints: Vec::new(),
            related: Vec::new(),
        }
    }
}

/// Diagnostic engine for collecting and reporting semantic errors and warnings
pub struct DiagnosticEngine {
    pub diagnostics: Vec<Diagnostic>,
    pub warnings_as_errors: bool,
    pub disable_all_warnings: bool,
}

impl Default for DiagnosticEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticEngine {
    pub fn new() -> Self {
        DiagnosticEngine {
            diagnostics: Vec::new(),
            warnings_as_errors: false,
            disable_all_warnings: false,
        }
    }

    pub fn report_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.level == DiagnosticLevel::Error)
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn report<T: IntoDiagnostic>(&mut self, error: T) {
        for diagnostic in error.into_diagnostic() {
            self.report_diagnostic(diagnostic);
        }
    }
}

pub trait IntoDiagnostic {
    fn into_diagnostic(self) -> Vec<Diagnostic>;
}

/// Configurable error formatter using annotate_snippets
pub struct ErrorFormatter {
    pub use_colors: bool,
}

impl Default for ErrorFormatter {
    fn default() -> Self {
        ErrorFormatter { use_colors: true }
    }
}

impl ErrorFormatter {
    pub fn new() -> Self {
        Self::default()
    }

    fn level<'a>(&self, diag: &Diagnostic) -> Level<'a> {
        match diag.level {
            DiagnosticLevel::Error => Level::ERROR,
            DiagnosticLevel::Warning => Level::WARNING,
            DiagnosticLevel::Note => Level::NOTE,
        }
    }

    /// Format a single diagnostic with rich source code context
    pub fn format_diagnostic(&self, diag: &Diagnostic, src: &str, filename: &str) -> String {
        let renderer = if self.use_colors {
            Renderer::styled()
        } else {
            Renderer::plain()
        };

        let start = diag.span.offset();
        let end = start + diag.span.len();

        let start = start.min(src.len());
        let end = end.min(src.len());

        let mut snippet = Snippet::source(src)
            .line_start(1)
            .path(filename)
            .fold(true)
            .annotation(
                AnnotationKind::Primary
                    .span(start..end)
                    .label(&diag.message),
            );

        for (span, msg) in &diag.related {
            let r_start = span.offset().min(src.len());
            let r_end = (r_start + span.len()).min(src.len());

            snippet = snippet.annotation(AnnotationKind::Context.span(r_start..r_end).label(msg));
        }

        let mut title = self.level(diag).primary_title(&diag.message);
        if let Some(code) = &diag.code {
            title = title.id(code);
        }

        let mut group = title.element(snippet);

        for hint in &diag.hints {
            group = group.element(Level::HELP.message(hint));
        }

        let report = vec![group];
        renderer.render(&report).to_string()
    }
}
