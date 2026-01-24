## 2025-02-21 - Metric Card Accessibility
**Learning:** Dashboard metric cards often use text characters like '↑' or '↓' for trends. Screen readers announce these literally (e.g., "Up Arrow"), lacking context.
**Action:** Replace text symbols with decorative icons (`aria-hidden="true"`) and provide explicit `sr-only` text (e.g., "Trending up") for context.

## 2025-05-23 - Modal Accessibility & Interaction
**Learning:** Confirmation modals often lack keyboard support (Escape to close) and semantic roles (`alertdialog`), making them traps for keyboard users and confusing for screen readers.
**Action:** Always add `keydown` listener for Escape, use `Teleport` for correct DOM placement, and ensure `role="alertdialog"` with `aria-labelledby`/`aria-describedby` are present.
