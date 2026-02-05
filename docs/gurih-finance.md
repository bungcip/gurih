# GurihFinance Documentation

## 1. Overview

**GurihFinance** is the core financial module of the GurihERP suite, built entirely on the **Gurih Framework**. It serves as the central ledger for all financial transactions, ensuring data integrity, auditability, and compliance with accounting standards.

It is designed to be **integration-first**, meaning other modules (Sales, Procurement, HR) do not write directly to the ledger but instead submit transactions via strictly defined **Posting Rules**.

### Role in GurihERP
- **Central Ledger**: All financial impact from other modules aggregates here.
- **Source of Truth**: The `JournalEntry` entity is the immutable record of financial history.
- **Compliance Engine**: Enforces rules like balanced transactions and open accounting periods at the framework level.

---

## 2. Architecture

GurihFinance leverages the **Gurih Framework**'s architecture, separating definition (DSL) from execution (Runtime).

### Project Structure

The module is defined by a set of `.kdl` files, each focusing on a specific domain aspect.

![Project Structure](images/finance-project-structure.png)

1.  **DSL Definition (`gurih-finance/*.kdl`)**: Defines entities (`Account`, `JournalEntry`), workflows, and rules.
2.  **Framework Engine (`gurih_runtime`)**: Loads the DSL, manages the database, and serves the API.
3.  **Module Logic (`FinancePlugin`)**: A Rust plugin that implements domain-specific constraints (e.g., `balanced_transaction`) and complex actions (e.g., `generate_closing_entry`).

### Execution Model

When the application starts:
1.  **Parser**: The framework reads `gurih.kdl` and included files (`coa.kdl`, `journal.kdl`, etc.).
2.  **Schema Generation**: It builds an in-memory schema and migrates the database (SQLite/Postgres).
3.  **Plugin Attachment**: The `FinancePlugin` attaches to hooks for validation and custom effects.
4.  **UI Generation**: The frontend automatically renders pages and dashboards based on the DSL.

![Finance Dashboard](images/finance-dashboard.png)

---

## 3. GurihFinance DSL

The behavior of GurihFinance is dictated by its DSL definitions. Below are the key constructs.

### Chart of Accounts (`coa.kdl`)

Accounts are the building blocks of the ledger. They are defined as hierarchical entities.

![Chart of Accounts List](images/finance-coa-list.png)

**Key DSL Features:**
- **`belongs_to "parent"`**: Enables a hierarchical Chart of Accounts (e.g., Assets -> Current Assets -> Cash).
- **`normal_balance`**: Enforces logical validation (e.g., Assets should have Debit balance).
- **`rule "PreventInUseAccountDelete"`**: Ensures data integrity by preventing deletion of accounts with existing journal entries.

### Journal & Workflow (`journal.kdl`)

The `JournalEntry` is the heart of the system. Its lifecycle is strictly controlled by a **Workflow**.

![Journal DSL Example](images/finance-dsl-example.png)

**Workflow Rules:**
- **`balanced_transaction #true`**: A custom precondition checked by `FinancePlugin`. It ensures `Sum(Debit) == Sum(Credit)`.
- **`period_open entity="AccountingPeriod"`**: Prevents posting into closed or locked periods.
- **`valid_parties #true`**: Ensures that for Control Accounts (like AR/AP), a valid Customer or Vendor is specified.
- **`immutable=#true`**: Once in the `Posted` state, the record cannot be modified.

![Journal Entry List](images/finance-journal-list.png)

### Accounting Periods (`period.kdl`)

Manages the fiscal timeline.

- **`status`**: Open, Closed, or Locked.
- **`generate_closing_entry`**: A complex action implemented in Rust that calculates Retained Earnings and creates a closing journal entry for the period.

### Reports (`reports.kdl`)

Reports are defined using the `query:flat` construct, which allows for performant read-only projections.

```kdl
query:flat "TrialBalanceQuery" for="Account" {
    params "start_date" "end_date"
    select "code"
    select "name"
    formula "total_debit" "SUM([debit])"
    formula "total_credit" "SUM([credit])"
    // ... joins and filters
}
```

---

## 4. End-to-End Example

### Scenario: Recording a Manual Transaction

1.  **Creation**: A user navigates to "Journal Entries" and clicks "New". They enter the date, description, and lines (Debits/Credits).
    *   *State*: `Draft`
2.  **Submission**: The user clicks "Post".
3.  **Validation**:
    *   The framework checks `requires { balanced_transaction #true }`.
    *   If Debits = 100 and Credits = 90, the transition fails with "Transaction not balanced".
4.  **Success**: If valid, the status updates to `Posted`. The UI now shows the record as Read-Only.

---

## 5. Integration Guide

External modules should **never** write to `JournalEntry` tables directly. Instead, they must use **Posting Rules**.

### Posting Rules (`integration.kdl`)

A `posting_rule` defines how a source document (e.g., `Invoice`, `PayrollRun`) maps to the general ledger.

![Integration DSL](images/finance-integration.png)

**Example: Payroll Integration**

When the HR module approves a payroll run, it triggers the `PayrollPosting` rule:

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
    // ...
}
```

**Benefits:**
- **Decoupling**: The HR module doesn't need to know about Account IDs or Debits/Credits logic.
- **Audit Trail**: The resulting Journal Entry is automatically linked to the `PayrollRun` ID.
- **Consistency**: Posting logic is defined in one place (the DSL), not scattered across codebases.
