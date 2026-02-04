# GurihSIASN Documentation

## 1. Overview

**GurihSIASN** (Sistem Informasi ASN) is the Human Resources management system for the Indonesian State Civil Apparatus (ASN), built on the Gurih Framework.

It manages the entire lifecycle of civil servants, from onboarding (CPNS) to retirement (Pensiun), including promotions, transfers, and leave management.

### Target Audience
- **Admin**: Manages master data (Jabatan, Golongan) and system configuration.
- **ASN (Pegawai)**: Views profile, requests leave, submits documents.
- **Auditor**: Reviews personnel history and compliance.

### Runtime View (Dashboard)

The dashboard provides a quick overview of personnel statistics, such as total ASN count and pending leave requests.

![SIASN Dashboard](images/siasn-dashboard.png)

---

## 2. DSL Usage in GurihSIASN

GurihSIASN utilizes the standard Gurih Framework DSL to model the complex lifecycle of a `Pegawai` (Employee) and other HR processes.

### Project Structure

The definition is split into domain-specific files (`kepegawaian.kdl`, `cuti.kdl`) and workflow definitions (`workflow.kdl`), all orchestrated by `app.kdl`.

![Project Structure](images/siasn-project-structure.png)

### Employee Lifecycle Workflow

The core logic is defined in `workflow.kdl`. Unlike traditional state machines, this workflow integrates domain-specific HR requirements and effects using the `workflow` construct.

![Workflow DSL](images/siasn-dsl-example.png)

**Key Components:**
- **`workflow "PegawaiStatusWorkflow"`**: Defines the lifecycle for the `Pegawai` entity on the `status_pegawai` field.
- **`requires`**: Preconditions that must be met before transition.
  - **`min_years_of_service`**: Checks employment duration (e.g., `min_years_of_service 1 from="tmt_cpns"`).
  - **`document`**: Verifies presence of required documents (e.g., `document "sk_pns"`).
  - **`min_age`**: Enforces age restrictions (e.g., `min_age 58 from="tanggal_lahir"`).
- **`effects`**: Side effects triggered upon transition.
  - **`update_rank_eligibility`**: Custom effect to flag employee for promotion.
  - **`suspend_payroll`**: Custom effect to stop salary payments (e.g., for Unpaid Leave or Retirement).
  - **`notify`**: Sends notifications to external systems (e.g., `notify "taspen"`).

### UI Generation (`kepegawaian.kdl`)

The UI for managing employees is also defined via DSL.

```kdl
page "DaftarPegawai" {
    title "Data Pegawai"
    datatable for="Pegawai" query="PegawaiQuery" {
        column "nip" label="NIP"
        column "nama" label="Nama"
        column "status_pegawai" label="Status"
        // ...
    }
}
```

![Pegawai List](images/siasn-pegawai-list.png)

---

## 3. System Flow

### Runtime Execution Example: Retirement (Pensiun)

1.  **Transition Request**:
    A user requests to move a `Pegawai` from `PNS` to `Pensiun` via the UI or API.

2.  **Validation (DSL Enforced)**:
    The framework checks the `requires` block in `workflow.kdl`:
    ```kdl
    transition "PNS_to_Pensiun" from="PNS" to="Pensiun" {
        requires {
             min_age 58 from="tanggal_lahir"
        }
        // ...
    }
    ```
    If the employee's age is less than 58, the transition is rejected with a validation error.

3.  **Effect Execution**:
    Upon successful validation, the effects are executed:
    ```kdl
        effects {
             suspend_payroll "true"
             notify "taspen"
        }
    ```
    - `suspend_payroll`: Updates the payroll flag in the database (or notifies the Finance module).
    - `notify`: Sends a message to the external Taspen system.

4.  **State Update**:
    The `status_pegawai` field is updated to `Pensiun`. Since this state is marked `immutable="true"`, further changes are restricted.

---

## 4. Comparison: GurihSIASN vs GurihFinance

While both modules use the Gurih Framework, they differ in their usage patterns, demonstrating the flexibility of the DSL.

| Feature | GurihFinance | GurihSIASN |
|---------|--------------|------------|
| **Core Entity** | `JournalEntry` (Immutable Ledger) | `Pegawai` (Mutable State Machine) |
| **Logic Focus** | Mathematical balance, integrity | Workflow rules, time-based eligibility |
| **DSL Style** | `rule`, `posting_rule` (declarative mapping) | `workflow` with custom effects (lifecycle definition) |
| **Validation** | `balanced_transaction` (strict equality) | `min_years_of_service`, `document` (policy checks) |

### Reusable Patterns

Both modules share common framework constructs:
- **`workflow`**: State transitions (used for Journal Status in Finance, Employee Status in SIASN).
- **`dashboard`**: Widget definitions for UI.
- **`role`**: RBAC permissions.
- **`query`**: Data extraction logic.
