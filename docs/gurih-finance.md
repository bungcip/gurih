# GurihFinance Documentation

## 1. Overview

**GurihFinance** is the core financial module of the GurihERP ecosystem. It provides a robust, double-entry accounting system driven entirely by the Gurih DSL.

### Role in GurihERP
GurihFinance acts as the central ledger for all financial transactions. Other modules (like HR, POS, Procurement) do not write directly to the ledger; instead, they submit transactions via **Posting Rules** which are validated and processed by GurihFinance.

### Key Features
- **Double-Entry Accounting**: Ensures debits equal credits for every transaction.
- **Configurable Chart of Accounts**: Hierarchical account structure defined in DSL.
- **Strict Validations**: Rules enforce positive balances, closed periods, and valid attributes.
- **Automated Posting**: Integration with other modules via `posting_rule`.
- **Real-time Reporting**: Financial statements generated on-the-fly.

## 2. Architecture

GurihFinance is built on the **Gurih Framework**, which separates the business definition (DSL) from the execution engine (Runtime).

### Architectural Diagram

```mermaid
graph TD
    subgraph "Gurih Framework"
        Engine[Runtime Engine]
        DB[(DataStore)]
    end

    subgraph "GurihFinance Module"
        DSL[DSL Files (*.kdl)]
        Rules[Business Rules]
        Reports[Report Definitions]
    end

    subgraph "External Modules"
        HR[GurihHR]
        POS[GurihPOS]
    end

    DSL -->|Loaded by| Engine
    Engine -->|Persists to| DB
    HR -->|Triggers| PostingRule
    POS -->|Triggers| PostingRule
    PostingRule -->|Creates| Journal[Journal Entry]
    Journal -->|Update| Ledger
```

### Project Structure

The project is structured as a standalone module containing DSL definitions:

```text
gurih-finance/
├── coa.kdl          # Chart of Accounts definition
├── journal.kdl      # Journal Entry entity & workflows
├── period.kdl       # Accounting Period management
├── reports.kdl      # Financial Report queries
├── integration.kdl  # Posting rules for external modules
└── gurih.kdl        # Main configuration & routing
```

## 3. GurihFinance DSL

GurihFinance uses the Gurih DSL to define its domain model. Below are the key constructs.

### 3.1 Chart of Accounts (`coa.kdl`)
Accounts are defined with strict types and normal balances.

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
    belongs_to "parent" entity="Account" // Hierarchy
}
```

![Chart of Accounts UI](images/finance-coa-list.png)

### 3.2 Journal Entries (`journal.kdl`)
The `JournalEntry` is the heart of the system. It supports a workflow from `Draft` to `Posted`.

**Transaction Lifecycle:**
1.  **Draft**: Created manually or via API. Editable.
2.  **Posted**: Validated (balanced, period open) and locked. Immutable.
3.  **Cancelled**: Voided entries.

```kdl
workflow "JournalWorkflow" for="JournalEntry" field="status" {
    state "Draft" initial=#true
    state "Posted" immutable=#true

    transition "post" {
        from "Draft" to "Posted"
        requires {
            balanced_transaction #true
            period_open entity="AccountingPeriod"
        }
    }
}
```

![Journal Entry List](images/finance-journal-list.png)

### 3.3 Accounting Periods (`period.kdl`)
Periods control when transactions can be posted.

```kdl
entity "AccountingPeriod" {
    field:string "name" // "Jan 2024"
    field:date "start_date"
    field:date "end_date"
    field:enum "status" "PeriodStatus" // Open, Closed, Locked
}
```

### 3.4 Reports (`reports.kdl`)
Reports are defined using the `query:flat` DSL, which aggregates data from the ledger.

```kdl
query:flat "TrialBalanceQuery" for="Account" {
    select "code"
    select "name"
    formula "total_debit" "SUM([debit])"
    formula "total_credit" "SUM([credit])"
    join "JournalLine" { ... }
}
```

![Trial Balance Report](images/finance-report-trial-balance.png)

## 4. End-to-End Example

### Step 1: Define an Account
```kdl
account "Cash" {
    code "101"
    type "Asset"
    normal_balance "Debit"
}
```

### Step 2: Create a Journal Entry
A user creates a journal entry to record a sale.

```kdl
// Conceptually
JournalEntry {
    description: "Cash Sale"
    date: "2024-01-01"
    lines: [
        { account: "Cash", debit: 100 },
        { account: "Sales Revenue", credit: 100 }
    ]
}
```

### Step 3: Posting
The user clicks "Post". The system checks:
- Is Total Debit == Total Credit? (Yes, 100 == 100)
- Is the period for "2024-01-01" open? (Yes)
- **Result**: Status becomes `Posted`. Record is immutable.

### Step 4: Reporting
The `TrialBalanceReport` now shows:
- Cash: 100 (Debit)
- Sales Revenue: 100 (Credit)

## 5. Integration Guide

Other modules integrate using `posting_rule`. This decouples the modules from Finance internals.

### Example: Invoice Posting

When an `Invoice` is created in the Sales module, a `JournalEntry` is automatically generated.

```kdl
posting_rule "InvoicePosting" for="Invoice" {
    description "\"Invoice #\" + doc.invoice_number"
    date "doc.date"

    entry {
        account "Accounts Receivable"
        debit "doc.total_amount"
    }

    entry {
        account "Sales Revenue"
        credit "doc.total_amount"
    }
}
```

**Integration Points:**
- **Trigger**: Document creation/update in external module.
- **Validation**: If the posting rule fails (e.g., account missing), the source transaction may be rejected or flagged.
