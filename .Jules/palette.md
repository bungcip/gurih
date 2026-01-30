# Palette's Journal

## 2025-02-12 - Generic Modals & ARIA
**Learning:** Generic container components like Modals often miss base ARIA roles (`dialog`, `alertdialog`) and `aria-modal="true"`. Developers focus on the "isOpen" logic but forget the semantic role.
**Action:** Always verify `Modal.vue` or similar wrapper components for `role="dialog"` and ensure they have a labelling mechanism (like `aria-labelledby` linked to a title). Use `useId` for robust ID generation.

## 2025-05-23 - Custom Select Keyboard Navigation
**Learning:** Custom select components (`role="combobox"`) often lack keyboard navigation (Arrow keys), relying only on mouse or Tab, which breaks the expected "native-like" experience and accessibility.
**Action:** Ensure all custom dropdowns implement `ArrowDown` (open/next), `ArrowUp` (prev), `Enter` (select), and `Escape` (close), with proper focus management (`trigger.focus()` on close).
