# GurihSIASN Documentation

## 1. Overview

**GurihSIASN** is a specialized module for managing the Indonesian Civil Service (ASN - Aparatur Sipil Negara) lifecycle. It is built on top of the Gurih Framework but introduces domain-specific constructs tailored for public sector HR management.

### Target Audience
*   **ASN**: For self-service (Cuti, Data updates).
*   **Admin/Verifikator**: For validating submissions (Kenaikan Pangkat, KGB).
*   **Auditors**: For compliance checks.

### Why DSL?
HR regulations in the public sector change frequently (e.g., changes in retirement age or leave policies). Hardcoding these rules would make the system rigid. Using a DSL allows regulators or functional analysts to update policy parameters (like `min_years_of_service`) in configuration files without engineering intervention.

---

## 2. DSL Usage in GurihSIASN

GurihSIASN heavily utilizes the `employee_status` DSL construct, which is a specialized state machine wrapper designed for HR lifecycles.

### 2.1 Employee Status Lifecycle (`status.kdl`)

This file defines the allowable transitions for an employee's career path (CPNS -> PNS -> Pensiun).

**Key Features:**
*   **Preconditions**: `min_years_of_service`, `document`.
*   **Effects**: `suspend_payroll`, `notify`.

```kdl
// Example: CPNS to PNS Transition
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

![DSL Editor](images/ide_status.png)

### 2.2 Workflows (`workflow.kdl`)

Standard workflows are used for transactional documents like Leave Requests (`Cuti`) or Periodic Salary Increases (`KGB`).

```kdl
workflow "KGBWorkflow" for="PengajuanKGB" field="status" {
    state "Draft" initial="true"
    state "Diajukan"
    state "Disetujui"
    state "Ditolak"

    transition "Draft_to_Diajukan" from="Draft" to="Diajukan" {
        requires {
             check_kgb_eligibility "pegawai"
        }
    }
}
```

### 2.3 Entity Definitions (`kepegawaian.kdl`)

Defines the complex data model of an ASN, including history tables (`Riwayat`).

```kdl
entity "Pegawai" {
    field:pk id
    field:string "nip" unique=#true
    field:name "nama"

    // Enum-driven status
    field:enum "status_pegawai" "StatusPegawai"

    // One-to-Many Relationships (Histories)
    has_many "riwayat_jabatan" "RiwayatJabatan"
    has_many "riwayat_unor" "RiwayatUnor"
}
```

![Pegawai List](images/siasn-pegawai-list.png)

---

## 3. System Flow

How a DSL definition translates to runtime behavior:

1.  **Definition**: An analyst updates `status.kdl` to require "2 years" instead of "1 year" for a transition.
2.  **Parsing**: On restart (or hot-reload), the `gurih_dsl` parser reads the new rule.
3.  **Framework**: The `WorkflowEngine` updates the state graph for the `Pegawai` entity.
4.  **Runtime**: When a user tries to click "Promote to PNS", the `check_precondition` logic in `HrPlugin` evaluates the new rule against the employee's data.

![SIASN Dashboard](images/siasn-dashboard.png)

---

## 4. Comparison: GurihSIASN vs GurihFinance

While both modules run on the same core, they utilize different patterns suitable for their domains.

| Feature | GurihFinance | GurihSIASN |
| :--- | :--- | :--- |
| **Primary Domain** | Accounting & Ledger | HR & Lifecycle |
| **Core Entity** | `Account`, `JournalEntry` | `Pegawai` |
| **State Management** | `workflow` (Transaction status) | `employee_status` (Lifecycle) + `workflow` |
| **Logic** | **Posting Rules** (Immutable ledger) | **Policy Rules** (Eligibility checks) |
| **DSL Specifics** | `posting_rule`, `query:flat` | `employee_status`, `check_kgb_eligibility` |
| **Immutability** | Strict (Posted Journals cannot change) | Flexible (History tables track changes) |

### Reusable Patterns
Both modules share:
*   **Entity/Field definitions**: Same syntax for defining data structures.
*   **UI Generation**: `page`, `form`, and `datatable` constructs work identically.
*   **Role-Based Access**: Permission system is unified in `app.kdl` and `gurih.kdl`.

This consistency allows developers to switch between modules without learning a new "language", only a new "dialect".
