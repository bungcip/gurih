# GurihFinance Documentation

## 1. Overview

**GurihFinance** is the accounting and financial management module of the GurihERP ecosystem. It provides a robust, double-entry bookkeeping system designed to be flexible and tightly integrated with other operational modules like HR (`GurihSIASN`), POS, and Procurement.

### Key Features
- **Double-Entry Ledger**: Ensures books always balance.
- **Configurable Chart of Accounts**: Supports hierarchy and grouping.
- **Sub-ledger Accounting**: Tracks "Parties" (Customers, Vendors, Employees) directly on journal lines.
- **Automated Workflows**: Journal entry approval processes defined in DSL.
- **DSL-Driven Reporting**: Define financial reports (Trial Balance, Balance Sheet) using declarative queries.

### Integration Role
GurihFinance acts as the central repository for financial data. Other modules generate financial impact by submitting transactions that `GurihFinance` validates and posts.

![Finance Dashboard](images/finance-dashboard.png)

## 2. Architecture

GurihFinance follows the standard **Gurih Framework** architecture, separating definition (DSL) from execution (Runtime).

### Structure
The project is organized to separate the business definition from the technical implementation:

- **DSL (`/gurih-finance/*.kdl`)**: Defines the *what*. Entities, Accounts, Rules, and Reports.
- **Runtime (`gurih_runtime`)**: The engine that parses KDL, builds the schema, and serves the API.
- **Plugins (`gurih_plugins`)**: Rust code that implements complex custom logic (e.g., specific posting rules) referenced by the DSL.

![Project Structure](images/finance-project-structure.png)

### Execution Model
1.  **Parsing**: The framework loads `gurih-finance/gurih.kdl` and all included KDL files.
2.  **Schema Generation**: Entities like `Account` and `JournalEntry` are converted into database tables.
3.  **Validation**: Rules like `LeafAccountOnly` are compiled into transition guards.
4.  **Runtime**: The API receives requests, validates them against the DSL rules, and persists data.

## 3. GurihFinance DSL

The behavior of GurihFinance is primarily defined in KDL files.

### 3.1 Chart of Accounts (`coa.kdl`)
Defines the structure of the General Ledger. Accounts can be organized hierarchically using the `parent` field.

![DSL in Editor](images/ide_coa.png)

**Key Constructs:**
- `entity "Account"`: The core definition of an account.
- `account "Name"`: Pre-defined system accounts.

```kdl
// Example from coa.kdl
account "Cash" {
    code "101"
    type "Asset"
    normal_balance "Debit"
}
```

**Constraints:**
- **Immutability**: Once an account has journal entries, it cannot be deleted (Rule: `PreventInUseAccountDelete`).
- **Hierarchy**: Posting is only allowed on "Leaf" accounts (Rule: `LeafAccountOnly`).

![Chart of Accounts UI](images/finance-coa-list.png)

### 3.2 Journal & Transaction Lifecycle (`journal.kdl`)
The `JournalEntry` entity is the heart of the system. It uses a **Workflow** to manage the lifecycle from `Draft` to `Posted`.

**Lifecycle:**
1.  **Draft**: Editable. No GL impact.
2.  **Posted**: Immutable. GL updated.
3.  **Cancelled**: Voided.

**DSL Workflow Definition:**
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

**Validation Rules:**
- `balanced_transaction`: Framework-provided check ensuring Total Debit = Total Credit.
- `period_open`: Ensures the transaction date falls within an Open accounting period.

![Journal Entry List](images/finance-journal-list.png)

### 3.3 Accounting Periods (`period.kdl`)
Manages fiscal periods to control when transactions can be posted.

- **Statuses**: `Open`, `Closed`, `Locked`.
- **Closing**: The `GenerateClosingEntry` action triggers the calculation of Retained Earnings.

### 3.4 Reports (`reports.kdl`)
Financial reports are defined using **Query DSL**. This allows for complex joins and aggregations without writing raw SQL.

**Example: Trial Balance Query**
```kdl
query:flat "TrialBalanceQuery" for="Account" {
    params "start_date" "end_date"
    select "code"
    select "name"
    formula "total_debit" "SUM([debit])"
    formula "total_credit" "SUM([credit])"

    // ... joins to JournalLine ...
}
```

## 4. End-to-End Example

### Step 1: Define Account
(Usually done once during setup)
```kdl
account "Office Supplies" {
    code "601"
    type "Expense"
    normal_balance "Debit"
}
```

### Step 2: Create Journal Entry
A user (or API) creates a `JournalEntry` in `Draft` status.

```json
POST /api/JournalEntry
{
  "date": "2026-02-05",
  "description": "Purchase of stationery",
  "lines": [
    { "account_id": "601", "debit": 100, "memo": "Pens and Paper" },
    { "account_id": "101", "credit": 100, "memo": "Cash payment" }
  ]
}
```

### Step 3: Post
The user triggers the `post` transition.
```json
POST /api/JournalEntry/1/transition/post
```
The framework checks:
1.  Debits (100) == Credits (100).
2.  Date is in an Open Period.
3.  Accounts are active and leaf nodes.

Result: Status changes to `Posted`.

### Step 4: Period Closing
At the end of the month, the accountant closes the period.

```json
POST /finance/periods/1/close
```
Logic executed:
- `status` updates to `Closed`.
- `GenerateClosingEntry` action runs to move Net Income to Retained Earnings.

### Step 5: Financial Report Generation
Generate a Trial Balance to verify the books.

```json
GET /api/reports/TrialBalance?start_date=2026-02-01&end_date=2026-02-28
```

Result:
| Code | Name | Total Debit | Total Credit |
| :--- | :--- | :--- | :--- |
| 101 | Cash | 0 | 100 |
| 601 | Office Supplies | 100 | 0 |

## 5. Integration Guide

External modules (like `GurihSIASN` for Payroll) integrate by submitting Journal Entries.

### Integration Points

![Integration Diagram](images/finance-integration.png)

1.  **API Integration**:
    Modules should POST to `/api/JournalEntry` with the `Draft` status, then immediately trigger `post` if automated, or leave for review.

2.  **Party Linking**:
    When creating a Journal Line for a specific employee (e.g., Payroll Advance), use the `party_type` and `party_id` fields.

    ```kdl
    // DSL Definition in JournalLine
    field:string "party_type" // "Pegawai"
    field:uuid "party_id"     // ID of the Pegawai
    ```

3.  **Error Handling**:
    If a rule is violated (e.g., Period Closed), the API returns a `400 Bad Request` with a descriptive message defined in the DSL rule (e.g., "Cannot post to a closed period").
