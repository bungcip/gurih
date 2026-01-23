## 2024-05-23 - Vue Reactivity & Expensive Instantiations
**Learning:** `v-for` loops in Vue templates that execute expensive constructor calls (like `new Intl.NumberFormat`) re-run those constructors on every render/update, causing significant performance overhead in large lists.
**Action:** Extract expensive formatters or computations into cached helpers or computed properties outside the render loop.
