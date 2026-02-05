# GurihSIASN Documentation

## 1. Overview

**GurihSIASN** (Sistem Informasi ASN) is the Human Resources management system for the Indonesian State Civil Apparatus (ASN), built on the Gurih Framework.

It manages the comprehensive lifecycle of civil servants, from onboarding (CPNS) to retirement (Pensiun), including promotions, transfers, and leave management.

### Target Audience
- **Admin**: Manages master data (Golongan, Jabatan, Unor) and oversees system configuration.
- **ASN (Pegawai)**: Views their profile, submits leave requests, and tracks their career history.
- **Auditor**: Reviews personnel history and compliance.

### Runtime View (Dashboard)

The dashboard provides real-time statistics, leveraging the framework's ability to aggregate data dynamically.

![SIASN Dashboard](images/siasn-dashboard.png)

---

## 2. DSL Usage in GurihSIASN

GurihSIASN utilizes the standard Gurih Framework DSL to model the complex lifecycle of a `Pegawai` (Employee) and other HR processes.

### Project Structure

The definition is organized by domain module (`kepegawaian`, `cuti`, `master`) and orchestrated by `app.kdl`.

![Project Structure](images/siasn-project-structure.png)

**Key Files:**
- **`app.kdl`**: The entry point, defining layout, menu, and RBAC roles.
- **`master.kdl`**: Defines reference data entities like `Golongan`, `Jabatan`, `Unor`.
- **`workflow.kdl`**: Defines the central state machine for Employee Status.
- **`kepegawaian.kdl`**: Defines the `Pegawai` entity and its UI (Forms/Lists).

### Employee Lifecycle Workflow

The core business logic is defined in `workflow.kdl`. It uses the standard **Workflow** construct to model the ASN lifecycle.

> **Note**: While the framework supports an `employee_status` syntactic sugar, GurihSIASN currently implements its logic using the explicit `workflow` construct for maximum control and clarity.

![Workflow DSL](images/siasn-dsl-example.png)

**Key Components:**
- **`workflow "PegawaiStatusWorkflow"`**: Manages the `status_pegawai` field on the `Pegawai` entity.
- **`requires`**: Preconditions for transitions.
  - **`min_years_of_service`**: e.g., `min_years_of_service 1 from="tmt_cpns"`.
  - **`document`**: e.g., `document "sk_pns"` (requires a file upload/link).
  - **`min_age`**: e.g., `min_age 58 from="tanggal_lahir"`.
- **`effects`**: Automated actions upon transition.
  - **`suspend_payroll "true"`**: Automatically flags the employee for payroll suspension (integrated via plugins).
  - **`notify`**: Sends notifications to external systems (e.g., Taspen, BKN).

---

## 3. System Flow

### Example: Retirement Process (Pensiun)

1.  **Transition Request**:
    A personnel officer initiates the "Pensiun" transition for an employee who is currently "PNS".

2.  **Validation (DSL Enforced)**:
    The runtime engine evaluates the `requires` block in `workflow.kdl`:
    ```kdl
    transition "PNS_to_Pensiun" from="PNS" to="Pensiun" {
        requires {
             min_age 58 from="tanggal_lahir"
        }
        // ...
    }
    ```
    If the employee is 57 years old, the transition is **blocked** immediately.

3.  **Effect Execution**:
    If valid, the system executes the defined effects:
    ```kdl
        effects {
             suspend_payroll "true"
             notify "taspen"
        }
    ```
    - **`suspend_payroll`**: A custom effect handled by `HrPlugin` to update the payroll eligibility flag.
    - **`notify "taspen"`**: Triggers an integration event to the pension fund system.

4.  **State Update**:
    The employee's status becomes `Pensiun`. This state is marked `immutable="true"`, preventing further status changes or edits to the record.

### UI Generation

The UI for managing employees is defined in `kepegawaian.kdl`.

![Pegawai List](images/siasn-pegawai-list.png)

```kdl
page "DaftarPegawai" {
    title "Master Data Pegawai"
    datatable for="Pegawai" {
        column "nip" label="NIP"
        column "nama" label="Nama"
        // ...
    }
}
```

---

## 4. Comparison: GurihSIASN vs GurihFinance

While both modules are built on the same framework, they demonstrate different architectural patterns.

| Feature | GurihFinance | GurihSIASN |
|---------|--------------|------------|
| **Core Entity** | `JournalEntry` (Immutable Ledger) | `Pegawai` (Mutable State Machine) |
| **Logic Focus** | Mathematical balance (`debit == credit`) | Time-based eligibility & Workflow rules |
| **Primary DSL** | `posting_rule` (declarative mapping) | `workflow` (state transitions) |
| **Validation** | Strict integrity (`balanced_transaction`) | Policy-based (`min_age`, `document`) |
| **State** | Linear (Draft -> Posted) | Cyclic/Complex (CPNS -> PNS -> Cuti -> PNS ...) |

### Reusable Patterns

Both modules share common framework capabilities:
- **RBAC**: `role "Admin"`, `role "Pegawai"` defined in DSL.
- **Dashboards**: Widget-based analytics defined in `.kdl`.
- **Data Tables**: Auto-generated CRUD views.
