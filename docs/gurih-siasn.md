# GurihSIASN Documentation

GurihSIASN is the Human Resources module of the GurihERP suite, specifically tailored for the Indonesian Civil Servant (ASN) system.

## 1. Overview

**GurihSIASN** manages the entire lifecycle of an employee (Pegawai), from recruitment (CPNS) to retirement (Pensiun).

*   **Users**: ASN employees, HR administrators, and auditors.
*   **Philosophy**: Uses a flexible workflow engine to accommodate complex and frequently changing government regulations (Perka BKN).
*   **DSL-Driven**: Policies are defined in `.kdl` files, allowing rapid updates without recompiling the core application.

## 2. DSL Usage in GurihSIASN

Unlike a standard CRUD application, GurihSIASN relies heavily on state machine definitions.

### Project Structure

```text
gurih-siasn/
├── app.kdl          # Application layout, menus, roles
├── cuti.kdl         # Leave management
├── kepegawaian.kdl  # Employee data definitions
├── master.kdl       # Master data (Golongan, Jabatan)
├── status.kdl       # Employee status transitions
└── workflow.kdl     # Approval workflows (KGB, SLKS)
```

![SIASN Dashboard](images/siasn-dashboard.png)
*Figure 1: SIASN Dashboard showing employee statistics.*

### 2.1. Employee Status (`status.kdl`)

The core of the system is the `employee_status` block, which defines the legal status of an employee.

**Key Concepts:**
*   **Transitions**: Allowable state changes (e.g., `CPNS` -> `PNS`).
*   **Preconditions**: Requirements that must be met before transition (e.g., `min_years_of_service`).
*   **Effects**: Side effects of the transition (e.g., `suspend_payroll`).

```kdl
// Example from status.kdl
employee_status "CPNS" for="Pegawai" field="status_pegawai" initial=#true {
    can_transition_to "PNS" {
        requires {
            min_years_of_service 1 from="tmt_cpns"
            document "sk_pns"
            valid_effective_date "tmt_pns"
        }
        effects {
            update_rank_eligibility "true"
            notify "unit_kepegawaian"
        }
    }
}
```

![Status DSL](images/ide_status.png)
*Figure 2: Employee Status definition in the DSL editor.*

### 2.2. Workflows (`workflow.kdl`)

Standard approval workflows for requests like Leave (Cuti) or Salary Increases (KGB).

```kdl
// Example from workflow.kdl
workflow "KGBWorkflow" for="PengajuanKGB" field="status" {
    state "Draft" initial="true"
    state "Diajukan"
    // ...

    transition "Draft_to_Diajukan" from="Draft" to="Diajukan" {
        requires {
             check_kgb_eligibility "pegawai"
        }
    }
}
```

## 3. System Flow

The system enforces rules at runtime:

1.  **Request**: An employee initiates a status change (e.g., submits SK PNS).
2.  **Validation**: The framework evaluates preconditions defined in the DSL.
    *   `min_years_of_service 1` checks the `tmt_cpns` field against the current date.
    *   `document "sk_pns"` ensures the file is uploaded.
3.  **Transition**: If valid, the state updates to `PNS`.
4.  **Effects**: The `update_rank_eligibility` effect is triggered, updating the employee's profile.

![Pegawai List](images/siasn-pegawai-list.png)
*Figure 3: Employee list reflecting current status.*

## 4. Comparison: GurihSIASN vs GurihFinance

While both modules use the Gurih Framework, their DSL usage differs:

| Feature | GurihFinance | GurihSIASN |
| :--- | :--- | :--- |
| **Focus** | Transactional integrity, double-entry accounting | Lifecycle management, regulatory compliance |
| **Workflows** | Simple (Draft -> Posted), immutable end state | Complex, multi-stage approvals, reversible states |
| **Validation** | Mathematical (Debits == Credits) | Policy-based (Years of Service, Document existence) |
| **DSL Patterns** | `account`, `journal`, `posting_rule` | `employee_status`, `workflow`, `can_transition_to` |

Future module developers should note that the framework supports both strict transactional models and flexible workflow models.
