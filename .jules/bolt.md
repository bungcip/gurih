## 2024-05-23 - Vue Reactivity & Expensive Instantiations
**Learning:** `v-for` loops in Vue templates that execute expensive constructor calls (like `new Intl.NumberFormat`) re-run those constructors on every render/update, causing significant performance overhead in large lists.
**Action:** Extract expensive formatters or computations into cached helpers or computed properties outside the render loop.

## 2026-01-31 - Missing Frontend Tests
**Learning:** The `web` directory lacks a test suite (no `test` script in `package.json`), making automated verification of UI performance optimizations impossible without setting up a test runner.
**Action:** Rely on `build` verification and manual code inspection. Tread carefully with logic changes.

## 2026-02-04 - MemoryDataStore String Cloning
**Learning:** `MemoryDataStore` was cloning string values for every record during filtering, causing O(N*M) allocations where N is record count and M is filter count.
**Action:** Refactored `find` and `count` to compare `&str` directly, resulting in ~30% performance improvement for string filters.
