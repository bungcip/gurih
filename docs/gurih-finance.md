# GurihFinance Documentation

GurihFinance is the core financial module of the GurihERP suite, designed to handle accounting, financial reporting, and transaction processing. It is built on the **Gurih Framework** and driven entirely by a Domain-Specific Language (DSL).

## 1. Overview

**GurihFinance** serves as the central ledger for all financial activities within the ERP system.

*   **Role**: Acts as the "source of truth" for financial data.
*   **Integration**: Receives transactions from other modules (e.g., HR, Sales, Procurement) via standardized integration points.
*   **Philosophy**: Implements a double-entry bookkeeping system where business logic is defined in DSL (`.kdl` files) and executed by the `Gurih Framework` runtime.

## 2. Architecture

GurihFinance follows the standard Gurih Framework architecture:

1.  **DSL Definitions**: Business rules, data structures, and workflows are defined in KDL files.
2.  **Framework Engine**: The Rust-based runtime parses these definitions and enforces rules (e.g., immutability, validation).
3.  **Module Logic**: Specific financial logic (like trial balance calculation) is implemented as a plugin or derived from generic engine capabilities.

### Project Structure

```text
gurih-finance/
├── coa.kdl          # Chart of Accounts definition
├── journal.kdl      # Journal Entry structure and workflows
├── period.kdl       # Accounting Period management
├── reports.kdl      # Financial Report queries and definitions
├── integration.kdl  # Rules for posting from other modules
└── gurih.kdl        # Module configuration
```

![Finance Dashboard](images/finance-dashboard.png)
*Figure 1: GurihFinance Dashboard showing key metrics.*

## 3. GurihFinance DSL

The behavior of the finance module is governed by the following DSL constructs.

### 3.1. Chart of Accounts (`coa.kdl`)

Defines the structure of the general ledger accounts. It supports hierarchical accounts via a self-referencing `parent` relationship.

**Key Features:**
*   **Account Types**: Asset, Liability, Equity, Revenue, Expense.
*   **Normal Balance**: Debit or Credit.
*   **Hierarchy**: Accounts can be groups or leaf nodes.

```kdl
// Example from coa.kdl
entity "Account" {
    field:pk id
    field:string "code" unique=#true
    field:string "name"
    field:enum "type" "AccountType"
    // ...
    belongs_to "parent" entity="Account"
}

account "Cash" {
    code "101"
    type "Asset"
    normal_balance "Debit"
}
```

![Chart of Accounts DSL](images/ide_coa.png)
*Figure 2: Chart of Accounts definition in the DSL editor.*

![Chart of Accounts UI](images/finance-coa-list.png)
*Figure 3: Runtime view of the Chart of Accounts.*

### 3.2. Journals and Ledgers (`journal.kdl`)

Manages the lifecycle of journal entries.

**Key Features:**
*   **Workflow**: `Draft` -> `Posted` (Immutable) or `Cancelled`.
*   **Validation**: Ensures total Debit equals total Credit before posting.
*   **Immutability**: Once `Posted`, entries cannot be modified.

```kdl
// Example from journal.kdl
workflow "JournalWorkflow" for="JournalEntry" field="status" {
    state "Draft" initial=#true
    state "Posted" immutable=#true

    transition "post" {
        from "Draft"
        to "Posted"
        requires {
            balanced_transaction #true
            period_open entity="AccountingPeriod"
        }
    }
}
```

![Journal Entry List](images/finance-journal-list.png)
*Figure 4: Journal Entries list showing status workflow.*

### 3.3. Accounting Periods (`period.kdl`)

Controls the fiscal periods to prevent posting to closed months.

**Status Workflow:** `Open` -> `Closed` -> `Locked`.

### 3.4. Reports (`reports.kdl`)

Defines financial reports using the `query:flat` construct.

```kdl
// Example from reports.kdl
query:flat "TrialBalanceQuery" for="Account" {
    select "code"
    select "name"
    formula "total_debit" "SUM([debit])"
    formula "total_credit" "SUM([credit])"
    // ...
}
```

## 4. End-to-End Example

### Scenario: Posting an Expense

1.  **Definition**: An account "Office Supplies" (Expense) exists in `coa.kdl`.
2.  **Creation**: A user creates a `JournalEntry` in `Draft` status via the UI or API.
    *   Line 1: Debit "Office Supplies" 500.00
    *   Line 2: Credit "Cash" 500.00
3.  **Posting**: The user triggers the `post` transition.
    *   The framework checks if the transaction is balanced.
    *   It verifies the `AccountingPeriod` for the date is `Open`.
    *   The status changes to `Posted`.
4.  **Reporting**: The `TrialBalanceQuery` now reflects these balances in the generated report.

## 5. Integration Guide

External modules (like `GurihSIASN` for payroll) integrate via `posting_rule` definitions in `integration.kdl`.

### Posting Rules

Modules define how their documents map to journal entries.

```kdl
// Example: Payroll Integration
posting_rule "PayrollPosting" for="PayrollRun" {
    description "\"Payroll for \" + doc.period_name"
    date "doc.payment_date"

    entry {
        account "Salaries Expense"
        debit "doc.total_gross_pay"
    }
    // ...
}
```

This decoupling ensures that the core finance module doesn't need to know the schema of the source documents, only how to map them.
