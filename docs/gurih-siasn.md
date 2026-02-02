# GurihSIASN Documentation

## 1. Overview

**GurihSIASN** (Sistem Informasi ASN) is the Human Resources management system for the Indonesian State Civil Apparatus (ASN), built on the Gurih Framework.

It manages the entire lifecycle of civil servants, from onboarding (CPNS) to retirement (Pensiun), including promotions, transfers, and leave management.

### Target Audience
- **Admin**: Manages master data (Jabatan, Golongan) and system configuration.
- **ASN (Pegawai)**: Views profile, requests leave, submits documents.
- **Auditor**: Reviews personnel history and compliance.

### Runtime View

![SIASN Dashboard](images/siasn-dashboard.png)

---

## 2. DSL Usage in GurihSIASN

GurihSIASN utilizes the standard Gurih Framework `workflow` DSL to model the complex lifecycle of a `Pegawai`.

### Project Structure

![Project Structure](images/siasn-project-structure.png)

### Employee Lifecycle Workflow

The core logic is defined in `workflow.kdl`. Unlike traditional state machines, this workflow integrates domain-specific HR requirements and effects.

![Workflow DSL](images/siasn-dsl-example.png)

**Key Components:**
- **`workflow "PegawaiStatusWorkflow"`**: Defines the lifecycle for the `Pegawai` entity on the `status_pegawai` field.
- **`requires`**: Preconditions that must be met before transition.
  - `min_years_of_service 1`: Checks employment duration.
  - `document "sk_pns"`: Verifies presence of required documents.
- **`effects`**: Side effects triggered upon transition.
  - `update_rank_eligibility`: Custom effect to flag employee for promotion.
  - `suspend_payroll`: Custom effect to stop salary payments (e.g., for Unpaid Leave or Retirement).
  - `notify`: Sends notifications to external systems.

### Domain-Specific Keywords

The DSL parser supports HR-specific keywords that translate to underlying framework actions:

| DSL Keyword | Framework Action | Purpose |
|-------------|------------------|---------|
| `suspend_payroll "true"` | `UpdateField(is_payroll_active, false)` | Stops payroll for unpaid leave/suspension. |
| `update_rank_eligibility "true"` | `UpdateField(rank_eligible, true)` | Marks employee as eligible for rank promotion. |

---

## 3. System Flow

### Runtime Execution

1. **DSL Loading**: The `app.kdl` and `workflow.kdl` files are loaded.
2. **Plugin Initialization**: The `HrPlugin` initializes and registers handlers for the custom effects (`suspend_payroll`).
3. **Transition Request**:
   - A user requests to move a `Pegawai` from `PNS` to `Pensiun` via the UI or API.

   ![Pegawai List](images/siasn-pegawai-list.png)

4. **Validation**:
   - Framework checks preconditions (e.g., `min_age 58` defined in `workflow.kdl`).
   - If `age < 58`, the transition is rejected with a validation error.
5. **Execution**:
   - Status updates to `Pensiun`.
   - `suspend_payroll` effect is executed, setting `is_payroll_active = false` in the database.
   - `notify "taspen"` sends a message to the pension system.

---

## 4. Comparison: GurihSIASN vs GurihFinance

While both modules use the Gurih Framework, they differ in their usage patterns.

| Feature | GurihFinance | GurihSIASN |
|---------|--------------|------------|
| **Core Entity** | `JournalEntry` (Immutable Ledger) | `Pegawai` (Mutable State Machine) |
| **Logic Focus** | Mathematical balance, integrity | Workflow rules, time-based eligibility |
| **DSL Style** | `rule`, `posting_rule` (declarative mapping) | `workflow` with custom effects (lifecycle definition) |
| **Validation** | `balanced_transaction` (strict equality) | `min_years_of_service`, `document` (policy checks) |

### Reusable Patterns

Both modules share:
- **`workflow`**: State transitions (used for Journal Status in Finance, Employee Status in SIASN).
- **`dashboard`**: Widget definitions for UI.
- **`role`**: RBAC permissions.
- **`report/query`**: Data extraction logic.
