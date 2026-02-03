# GurihFinance Documentation

## 1. Overview

**GurihFinance** is the financial accounting module of the GurihERP suite. It provides a robust, double-entry bookkeeping system designed to be extensible and integrated with other operational modules (such as HR, Procurement, and POS).

Built on top of the **Gurih Framework**, it leverages a DSL-driven architecture to define its data models, accounting rules, and reporting structures. This ensures that financial policies are defined declaratively in code (`.kdl` files) rather than hardcoded in the application logic.

### Key Features
- **Double-Entry Ledger**: Ensures all transactions are balanced.
- **Configurable Chart of Accounts**: Hierarchical account structure with support for Assets, Liabilities, Equity, Revenue, and Expenses.
- **Journal Entry Workflow**: Strict lifecycle management (Draft -> Posted -> Cancelled) with immutability guarantees.
- **Automated Integration**: API for other modules to post transactions via `posting_rule`.
- **Flexible Reporting**: DSL-based query engine for Trial Balance, Balance Sheet, and Income Statement.

---

## 2. Architecture

GurihFinance follows the standard Gurih Framework architecture, separating the definition (DSL) from the execution (Runtime).

### Structure

The module is organized as a collection of KDL files that define the schema and behavior. The main entry point is `gurih.kdl`, which includes other functional definitions.

![Project Structure](images/finance-project-structure.png)

### Execution Model

1. **DSL Definition**: The schema is defined in `gurih.kdl` and included files (`coa.kdl`, `journal.kdl`, `period.kdl`, `reports.kdl`, `integration.kdl`).
2. **Compiler**: The `gurih_dsl` crate parses these files into an Intermediate Representation (IR).
3. **Runtime**: The `gurih_runtime` loads the IR and initializes the database (SQLite/Postgres).
4. **Plugin Logic**: The `FinancePlugin` (written in Rust) attaches to the runtime to enforce specific logic, such as:
   - Validating `balanced_transaction` on posting.
   - Performing period closing calculations (`finance:generate_closing_entry`).
   - Reversing journals (`finance:reverse_journal`).

### Runtime Output (Dashboard)

When the system runs, it loads the DSL and initializes the database and API endpoints automatically.

![Finance Dashboard](images/finance-dashboard.png)

---

## 3. GurihFinance DSL

The core of GurihFinance is its DSL definitions.

### Chart of Accounts (`coa.kdl`)

Accounts are defined as entities with specific fields like `code`, `type`, and `normal_balance`. The hierarchy is managed via a self-referencing `parent` field.

```kdl
enum "AccountType" {
    Asset
    Liability
    Equity
    Revenue
    Expense
}

entity "Account" {
    field:pk id
    field:string "code" unique=#true
    field:string "name"
    field:enum "type" "AccountType"
    field:enum "normal_balance" "NormalBalance"
    field:boolean "is_active" default="true"
    field:boolean "is_group" default="false"

    // Using self-referencing relationship for hierarchy
    belongs_to "parent" entity="Account"
}
```

**UI Representation:**
The framework automatically generates the UI for managing accounts based on the entity definition.

![Chart of Accounts List](images/finance-coa-list.png)

### Journal Entries (`journal.kdl`)

Transactions are recorded as `JournalEntry` records containing multiple `JournalLine` items. The workflow is strictly controlled via DSL.

![Journal Entry DSL](images/finance-dsl-example.png)

**Key Constructs:**
- **`workflow "JournalWorkflow"`**: Defines the state machine (Draft, Posted, Cancelled).
- **`requires { balanced_transaction #true }`**: A custom validator enforced by `FinancePlugin` that ensures `sum(debit) == sum(credit)` before the transition to "Posted" is allowed.
- **`period_open entity="AccountingPeriod"`**: Checks if the transaction date falls within an open accounting period.
- **`state "Posted" immutable=#true`**: Prevents editing the journal entry once it reaches the "Posted" state.

### Accounting Periods (`period.kdl`)

Periods manage the financial timeline. The `generate_closing_entry` action is used to close a period, calculating retained earnings.

```kdl
entity "AccountingPeriod" {
    field:pk id
    field:string "name" // e.g. "Jan 2024"
    field:date "start_date"
    field:date "end_date"
    field:enum "status" "PeriodStatus" default="Open"
}

action "GenerateClosingEntry" {
    params "id"
    step "finance:generate_closing_entry" period_id="param(\"id\")"
}
```

### Reporting (`reports.kdl`)

Reports are defined using the `query:flat` construct, which allows joining entities and calculating aggregates.

```kdl
query:flat "TrialBalanceQuery" for="Account" {
    params "start_date" "end_date"
    select "code"
    select "name"

    // Aggregate formulas
    formula "total_debit" "SUM([debit])"
    formula "total_credit" "SUM([credit])"

    join "JournalLine" {
        select "debit"
        select "credit"
        // ...
    }
    // ...
}
```

---

## 4. End-to-End Example

### 1. Journal Entry Creation
A user creates a journal entry via the generated UI or API. It starts in the **Draft** state.

![Journal Entry List](images/finance-journal-list.png)

### 2. Validation & Posting
When the user attempts to post the journal, the `post` transition is triggered.

```kdl
    transition "post" {
        from "Draft"
        to "Posted"
        requires {
            balanced_transaction #true
            period_open entity="AccountingPeriod"
        }
    }
```

The system verifies:
1.  **Balance**: Total Debit equals Total Credit.
2.  **Period**: The transaction date is in an "Open" period.

If valid, the status changes to **Posted**, and the record becomes immutable.

---

## 5. Integration Guide

Other modules (like `GurihSIASN` for Payroll or a POS system) integrate with Finance using **Posting Rules**.

### Posting Rules (`integration.kdl`)

External modules do not write to `JournalEntry` directly. Instead, they define a `posting_rule` that maps their documents to accounting entries.

![Integration DSL](images/finance-integration.png)

**Example: Payroll Integration**

When a `PayrollRun` is approved in the HR module, it triggers the `PayrollPosting` rule:

```kdl
posting_rule "PayrollPosting" for="PayrollRun" {
    description "\"Payroll for \" + doc.period_name"
    date "doc.payment_date"

    entry {
        account "Salaries Expense"
        debit "doc.total_gross_pay"
    }

    entry {
        account "Tax Payable"
        credit "doc.total_tax"
    }

    entry {
        account "Cash"
        credit "doc.total_net_pay"
    }
}
```

**Workflow:**
1.  **Trigger**: Logic in the external module (or a manual action) invokes the posting rule.
2.  **Mapping**: The framework evaluates the expressions (e.g., `doc.total_gross_pay`) against the source document.
3.  **Creation**: A balanced `JournalEntry` is automatically created and posted.
4.  **Audit**: The generated journal entry is linked back to the source `PayrollRun` ID for auditability.
