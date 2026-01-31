# GurihSIASN Documentation

## 1. Overview

**GurihSIASN** (Sistem Informasi ASN) is a comprehensive Human Resource Management system designed for Indonesian government agencies. It is built on the **Gurih Framework**, utilizing a powerful DSL to manage the complex lifecycle of civil servants (ASN).

### Target Audience
- **ASN (Pegawai):** To view their profile, history, and submit leave requests.
- **Admin/HR:** To manage master data, process mutations, and handle certifications.

## 2. DSL Usage in GurihSIASN

GurihSIASN extensively uses the **Gurih DSL** to model the intricacies of staffing (Kepegawaian).

### Domain Entities
The core entity is `Pegawai` (Employee), with rich relationships to history tables (`Riwayat*`).

```kdl
entity "Pegawai" {
    field:pk id
    field:string "nip" unique=#true
    field:name "nama"
    field:enum "status_pegawai" "StatusPegawai" // CPNS, PNS, Pensiun, etc.

    // Relationships to Master Data
    belongs_to "Jabatan"
    belongs_to "Unor" // Unit Organisasi
    belongs_to "Golongan"
}
```

![Pegawai List](images/siasn-pegawai-list.png)

### Workflow & Transitions (`workflow.kdl`)
The lifecycle of an employee is strictly controlled by workflow definitions. Transitions between statuses (e.g., CPNS to PNS) require specific documents and conditions.

```kdl
employee_status "CPNS" for="Pegawai" field="status_pegawai" {
    can_transition_to "PNS" {
        requires {
             min_years_of_service 1 from="tmt_cpns"
             document "sk_pns"
        }
        effects {
             update "rank_eligible" "true"
             notify "unit_kepegawaian"
        }
    }
}
```

### Dashboard
The dashboard aggregates key statistics using DSL query widgets.

![SIASN Dashboard](images/siasn-dashboard.png)

## 3. System Flow

1.  **DSL Definition**: Policies are defined in `workflow.kdl` and `kepegawaian.kdl`.
2.  **Runtime Execution**: The Gurih Runtime loads these definitions.
    - **HrPlugin**: A specialized plugin (`gurih_runtime::hr_plugin`) handles custom logic like `suspend_payroll` or `update_rank_eligibility`.
3.  **User Interaction**: Users interact with the system via the generic web UI, which dynamically renders forms and tables based on the DSL.

![System Architecture](images/siasn-project-structure.png)

## 4. Comparison: GurihSIASN vs GurihFinance

While both modules run on the same core framework, they demonstrate different strengths of the DSL:

| Feature | GurihFinance | GurihSIASN |
| :--- | :--- | :--- |
| **Primary Focus** | Transactional Integrity, Ledgers | Lifecycle Management, Compliance |
| **Key DSL Feature** | `posting_rule`, `balanced_transaction` | `employee_status` workflow, `requires` |
| **Plugin Logic** | `FinancePlugin` (Double Entry) | `HrPlugin` (Payroll Status, Eligibility) |
| **Reporting** | Financial Statements (Balance Sheet) | Operational Stats (Gender, Status) |

Both share the same **UI components**, **Authentication**, and **Permission** models defined in the core framework.
