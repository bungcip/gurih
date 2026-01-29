# Palette's Journal

## 2025-02-12 - Generic Modals & ARIA
**Learning:** Generic container components like Modals often miss base ARIA roles (`dialog`, `alertdialog`) and `aria-modal="true"`. Developers focus on the "isOpen" logic but forget the semantic role.
**Action:** Always verify `Modal.vue` or similar wrapper components for `role="dialog"` and ensure they have a labelling mechanism (like `aria-labelledby` linked to a title). Use `useId` for robust ID generation.
