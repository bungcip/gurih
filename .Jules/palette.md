# Palette's Journal

## 2025-02-12 - Generic Modals & ARIA
**Learning:** Generic container components like Modals often miss base ARIA roles (`dialog`, `alertdialog`) and `aria-modal="true"`. Developers focus on the "isOpen" logic but forget the semantic role.
**Action:** Always verify `Modal.vue` or similar wrapper components for `role="dialog"` and ensure they have a labelling mechanism (like `aria-labelledby` linked to a title). Use `useId` for robust ID generation.

## 2025-05-23 - Custom Select Keyboard Navigation
**Learning:** Custom select components (`role="combobox"`) often lack keyboard navigation (Arrow keys), relying only on mouse or Tab, which breaks the expected "native-like" experience and accessibility.
**Action:** Ensure all custom dropdowns implement `ArrowDown` (open/next), `ArrowUp` (prev), `Enter` (select), and `Escape` (close), with proper focus management (`trigger.focus()` on close).

## 2025-05-24 - Semantic Steppers
**Learning:** Stepper components are often built with `div`s, losing semantic meaning (list order) and accessibility (keyboard nav, current step).
**Action:** Use `<ol>` and `<li>` for ordered steps. Ensure clickable steps have `tabindex="0"`, `focus` styles, and keyboard handlers (`Enter`/`Space`). Use `aria-current="step"` for the active step.

## 2025-05-24 - Vue 3 Attribute Inheritance in Wrappers
**Learning:** Custom input wrappers (like `CurrencyInput.vue`) often default to `inheritAttrs: true`, applying `required`, `name`, and `aria-label` to the root `div` instead of the `input`. This breaks form validation and accessibility.
**Action:** Use `defineOptions({ inheritAttrs: false })` and bind `$attrs` to the inner native control (`<input v-bind="$attrs" />`) for all form components wrapping native inputs.

## 2025-05-24 - Button Icon Consistency
**Learning:** Components often define props (like `icon`) that are partially implemented or ignored in favor of slots, leading to confusing APIs and broken UI when developers trust the prop types.
**Action:** Ensure "convenience props" (like `icon`, `label`) are fully functional and backed by the appropriate sub-components. Support standard variations (like `iconPosition`) to reduce the need for custom slot boilerplate.
