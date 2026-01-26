## 2024-05-22 - Icon-Only Button Accessibility Pattern
**Learning:** Icon-only buttons (like password toggle or action icons) in this codebase often lack `aria-label`, making them invisible to screen readers despite visual cues.
**Action:** Always verify `aria-label` is present on buttons that don't have text content, especially in form components like `Password` or `FileUpload`.
