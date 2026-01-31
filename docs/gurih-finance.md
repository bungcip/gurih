# GurihFinance Documentation

## 1. Overview

**GurihFinance** is the financial accounting module of the **GurihERP** ecosystem. Built entirely on the **Gurih Framework**, it leverages a domain-specific language (DSL) to define accounting rules, charts of accounts, and transaction workflows.

### Role in GurihERP
GurihFinance serves as the central ledger for all financial transactions. Other modules such as **GurihSIASN** (HR/Payroll), POS, and Procurement integrate with Finance to automatically post journal entries based on business events.

## 2. Architecture

GurihFinance follows the standard **Gurih Framework** architecture:

1.  **DSL Definitions (`*.kdl`)**: The business logic, data structures, and workflows are defined in KDL files.
2.  **Gurih Runtime**: The Rust-based engine parses the DSL and executes the application logic, serving both the API and the UI configuration.
3.  **Module Logic**: Specific financial rules (like double-entry validation) are enforced by the `FinancePlugin` within the runtime.

![Project Structure](images/finance-project-structure.png)

## 3. GurihFinance DSL

The core of GurihFinance is defined in the `gurih-finance/` directory.

### Chart of Accounts (`coa.kdl`)
Defines the structure of the general ledger. Accounts can be grouped and hierarchical.

```kdl
entity "Account" {
    field:pk id
    field:string "code" unique=#true
    field:string "name"
    field:enum "type" "AccountType" // Asset, Liability, Equity, etc.
    field:enum "normal_balance" "NormalBalance" // Debit, Credit

    // Hierarchy
    belongs_to "parent" entity="Account"
}

// Pre-defined Account
account "Cash" {
    code "101"
    type "Asset"
    normal_balance "Debit"
}
```

![Chart of Accounts UI](images/finance-coa-list.png)

### Journal Entries & Rules (`journal.kdl`)
Transactions are recorded as `JournalEntry` records containing multiple `JournalLine` items.

**Validation Rules:**
- **Positive Amounts:** Debits and Credits must be non-negative.
- **Leaf Accounts Only:** You cannot post to a group account.
- **Balanced Transaction:** Total Debit must equal Total Credit (enforced by `FinancePlugin`).

```kdl
workflow "JournalWorkflow" for="JournalEntry" field="status" {
    state "Draft" initial=#true
    state "Posted" immutable=#true
    state "Cancelled" immutable=#true

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

### Integration Rules (`integration.kdl`)
Defines how external documents map to journal entries.

```kdl
posting_rule "PayrollPosting" for="PayrollRun" {
    description "\"Payroll for \" + doc.period_name"
    entry {
        account "Salaries Expense"
        debit "doc.total_gross_pay"
    }
    entry {
        account "Cash"
        credit "doc.total_net_pay"
    }
}
```

## 4. End-to-End Example

### 1. Account Definition
Users define accounts via the DSL or the UI. The `AccountForm` page allows creating new accounts.

### 2. Creating a Journal Entry
A user creates a new Journal Entry in "Draft" status.

![Journal Entry Form](images/finance-journal-list.png)

### 3. Posting
When the user clicks "Post", the `JournalWorkflow` is triggered. The system validates:
1.  The transaction is balanced.
2.  The accounting period is open.
3.  No rules are violated.

If successful, the status changes to `Posted`, and the record becomes immutable.

### 4. Reporting
Financial reports like the **Trial Balance** aggregate these posted entries.

![Finance Dashboard](images/finance-dashboard.png)

## 5. Integration Guide

To integrate a new module with GurihFinance:

1.  **Define a `posting_rule`** in `integration.kdl` referencing your source entity (e.g., `Invoice`).
2.  **Trigger the posting** via a workflow transition or action in your module using the `post_journal` effect.

Example:
```kdl
// In your module's workflow
transition "approve" {
    effects {
        post_journal "InvoicePosting" // Matches the rule name
    }
}
```
