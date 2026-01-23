## 2025-02-21 - Metric Card Accessibility
**Learning:** Dashboard metric cards often use text characters like '↑' or '↓' for trends. Screen readers announce these literally (e.g., "Up Arrow"), lacking context.
**Action:** Replace text symbols with decorative icons (`aria-hidden="true"`) and provide explicit `sr-only` text (e.g., "Trending up") for context.
