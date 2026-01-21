# GurihERP DSL Design

This document outlines the KDL-based DSL schemas for GurihERP.

## 1. Core Structure
Every GurihERP project starts with global settings.

```kdl
name "GurihERP"
version "0.1.0"

persistence { // Alias: database, datastore
    type "postgres" // or "sqlite"
    url "env:DATABASE_URL"
}

storage "default" {
    driver "local"
    location "./storage"
}

icons {
    "home" "lucide:home"
    "user" "lucide:user"
    "settings" "lucide:settings"
    "trash" "lucide:trash-2"
    "pencil" "lucide:pencil"
    // Usage in other components: icon="home"
}
```

## 2. Module System
Modules encapsulate related entities, enums, and workflows.

```kdl
module "Voucher" {
    // Contents of the module
}
```

## 3. Data Modeling

GurihERP membedakan antara **Physical Schema** (`table`) dan **Business Schema** (`entity`).

### 3.1 Table (Optional)
`table` mendefinisikan struktur database secara teknis. Jika tidak didefinisikan secara eksplisit, sistem akan membuat table secara otomatis berdasarkan `entity`.

```kdl
table "products" {
    column "id" type="serial" primary=true
    column "code" type="varchar" len=50 unique=true
    column "name" type="varchar" len=255
    column "price_decimal" type="decimal" precision="15,2"
    column "created_at" type="timestamp"
}
```

### 3.2 Entity
`entity` adalah model bisnis utama. Tipe datanya tidak menggunakan istilah teknis (`varchar`, `int`), melainkan menggunakan tipe data yang bermakna bisnis (**Semantic Types**).

**Semantic Types:**
- `field:pk`: Primary key (minimal ada satu di tiap entity).
- `field:serial`: Human-readable unique identifier (e.g., INV/2024/001). Usually linked to a `serial_generator`.
- `field:sku`: Specialized code for inventory/items.
- `field:name`: Nama orang atau barang. (hanya boleh ada satu di tiap entity)
- `field:title`: Judul atau label pendek. (hanya boleh ada satu di tiap entity)
- `field:description`: Deskripsi atau penjelasan panjang. (hanya boleh ada satu di tiap entity)
- `field:avatar`: Foto dari entity (hanya boleh ada satu di tiap entity).
- `field:money`: Nilai mata uang.
- `field:email` / `field:phone` / `field:address`: Data kontak.
- `field:password`: Password.
- `field:enum`: Keadaan (biasanya merujuk ke Enum).
- `field:int` / `field:float`: Angka.
- `field:date` / `field:timestamp`: Waktu.
- `field:string`: String pendek biasa.
- `field:text`: Penjelasan panjang biasa (textarea).
- `field:image`: Gambar biasa.
- `field:file`: File biasa (requires `storage` property).

### 3.2.1 Storage vs Persistence
GurihERP distinguishes between **Persistence** (data stored in a database) and **Storage** (files or documents stored in a file system or S3).

- **Persistence/DataStore**: Where your entities and their fields are stored. Configured using `persistence { ... }`.
- **Storage**: Where files (images, documents) are stored. Configured using `storage "name" { ... }`.

Example:
```kdl
storage "s3_public" {
    driver "s3"
    props bucket="my-bucket" region="us-east-1"
}

entity "User" {
    field:pk id
    field:avatar "photo" storage="s3_public"
}
```

> [!TIP]
> You can use `nullable=#true` to explicitly mark a field as optional (which translates to `required=false`). By default, fields are optional unless `required=#true` is set.

### 3.3 Serial Generator
Serial generators define rules for creating automatic human-readable codes.

```kdl
serial_generator "InvoiceCode" {
    prefix "INV/"
    date "YYYY/"
    sequence digits=4
    // Result: INV/2024/0001
}
```

### 3.4 Entity Example with Code Grouping
```kdl
entity "Invoice" {
    field:pk id
    field:serial "invoice_number" serial_generator="InvoiceCode"
    field:date "invoice_date"
    field:money "total_amount"
}
```

### 3.5 Relationships (Business Level)
Relationships describe how entities allow business processes to flow. We avoid low-level terms like `foreign_key`.

Supported Relations:
- `belongs_to`: The entity "owns" the link (e.g., Invoice belongs to Customer).
- `has_many`: The entity has a collection of items (e.g., Customer has many Invoices).
- `has_one`: One-to-one relationship.

Example:
```kdl
entity "Order" {
    field:pk id
    // Creates a "customer" field in logic, maps to "customer_id" in DB.
    belongs_to "Customer" 
    
    // Explicit field name:
    // belongs_to "shipping_address" entity="Address"
}

entity "Customer" {
    field:pk id
    // Allows accessing `customer.orders` in logic/expressions
    has_many "orders" "Order"
}
```

### 3.3 Enums
Enums for static lists of options.

```kdl
enum "OrderStatus" {
    Draft
    Pending
    Processing
    Completed
    Cancelled
}
```

### 3.6 Advanced Entity Options
Advanced ERP behaviors often inspired by Frappe/NextERP.

- `is_submittable`: If true, document can be "Submitted" making it immutable.
- `track_changes`: Automatically logs field changes (Audit Trail).
- `is_single`: For settings pages (stores only one row in DB).

```kdl
entity "Invoice" {
    // ... fields ...
    
    options {
        is_submittable #true
        track_changes #true
    }
}

entity "SystemSettings" {
    options {
        is_single #true
    }
    field:string "company_name"
    field:string "default_currency"
}
```

### 3.7 Composition (Child Tables)
For strong "Whole-Part" relationships (e.g., Invoice Items), use `child_table`.
This implies:
- Cascading delete (delete Invoice -> delete Items).
- Items are saved/validated together with Parent.
- Usually displayed as an editable grid inside the Parent form.

```kdl
entity "InvoiceItem" {
    // No direct routing or page usually needed
    field:pk id
    field:string "item_name"
    field:money "amount"
}

entity "Invoice" {
    // ...
    has_many "items" "InvoiceItem" type="composition"
}
```

### 3.8 Workflows
Workflows define state transitions for entities, useful for approval processes or lifecycles.

```kdl
workflow "LeaveWorkflow" for="LeaveRequest" field="status" {
    state "Draft" initial="true"
    state "Pending"
    state "Approved"
    state "Rejected"
    state "Cancelled"

    transition "submit" {
        from "Draft"
        to "Pending"
    }
    
    transition "approve" {
        from "Pending"
        to "Approved"
    }

    transition "reject" {
        from "Pending" 
        to "Rejected"
    }
    
    transition "cancel" {
        from "Draft" "Pending"
        to "Cancelled"
    }
}
```

### 3.9 Actions
Actions encapsulate business logic that can be triggered from the UI or API. They can be defined at the top level or within a `module`.

```kdl
action "DeletePosition" {
    params "id"
    step "entity:delete" target="Position" id="param(\"id\")"
}
```

**Actions in Modules:**
```kdl
module "Organization" {
    // ... entities ...

    action "PromoteEmployee" {
        params "employee_id" "new_position_id"
        step "entity:update" target="Employee" id="param(\"employee_id\")" {
            position_id="param(\"new_position_id\")"
        }
    }
}
```

## 4. Routing
Routes map URL paths to UI components. Each route or group can be protected by a `permission`.

```kdl
routes {
    route "/" to="MainDashboard" layout="AdminLayout"
    
    group "/inventory" permission="inventory.view" layout="AdminLayout" {
        route "/products" to="Inventory.ProductList"
        route "/products/new" to="Inventory.ProductForm" permission="inventory.create"
        route "/products/:id" to="Inventory.ProductForm" permission="inventory.edit"
    }

    group "/auth" layout="AuthLayout" {
        route "/login" to="Auth.Login"
    }

    group "/sales" permission="sales.view" {
        route "/customers" to="Sales.CustomerList"
        route "/orders" to="Sales.OrderList"
    }

    // Route to Action
    group "/positions" {
        route:delete "/:id" action="DeletePosition"
    }
}
```

## 5. UI and Navigation

### 5.0 Layout
Layouts define the common structure (sidebar, header, footer) that wraps pages.

```kdl
layout "AdminLayout" {
    header {
        search_bar true
        user_menu true
    }
    sidebar {
        menu_ref "MainMenu" // References a defined menu
    }
    footer "DefaultFooter"
}

layout "AuthLayout" {
    // minimalist layout for login/register
    header false
    sidebar false
}
```

### 5.1 Page
Pages define the user interface. A page can contain components like `datatable`, `form`, or custom widgets.

```kdl
page "ProductList" {
    title "List Produk"
    layout "AdminLayout" // Optional, default can be set globally
    
    datatable for="Product" {
        column "sku" label="SKU"
        column "name" label="Nama Barang"
        column "price" label="Harga"
        column "stock_quantity" label="Stok"
        
        // Actions should point to routes, allowing properly scoped permissions and context
        action "Edit" icon="pencil" to="/inventory/products/:id"
        action "Delete" icon="trash" variant="danger"
    }
}

page "ProductForm" {
    title "Form Produk"
    form for="Product" {
        field "sku"
        field "name"
        field "description" type="textarea"
        
        group {
            field "price"
            field "stock_quantity"
        }
        
        submit "Simpan"
    }
}
```

### 5.2 Datatable
Used for listing data. It can be standalone in a page or embedded in a form (for child entities).

Attributes:
- `for`: The entity name.
- `searchable`: Boolean.
- `sortable`: Boolean.

### 5.3 Form
Used for data entry.

Attributes:
- `for`: The entity name.
- `layout`: "vertical" | "horizontal".

Inner elements:
- `field`: Maps to entity attributes.
- `section`: Groups fields with a title.
- `group`: Horizontal layout for fields.

### 5.4 Dashboard
Dashboards are specialized pages for high-level overviews using widgets.

```kdl
dashboard "MainDashboard" {
    title "Ringkasan Bisnis"
    
    grid columns=4 {
        widget "TotalSales" type="stat" {
            label "Total Penjualan"
            value "sum:Order.total"
            icon "dollar-sign"
            trend "up" 12 // 12% increase
        }
        
        widget "PendingOrders" type="stat" {
            label "Pesanan Pending"
            value "count:Order[status=Pending]"
            icon "clock"
        }
    }
    
    row {
        widget "SalesChart" type="chart" {
            title "Tren Penjualan"
            query "select date, sum(total) from orders group by date"
            chart_type "line"
        }
        
        widget "RecentActivities" type="list" {
            title "Aktivitas Terbaru"
            for "AuditLog" limit=5
        }
    }
}
```

### 5.5 Menu
Defines the sidebar/navigation structure. Menu items point to paths defined in `routes`.

```kdl
menu {
    item "Dashboard" to="/" icon="home"
    group "Sales" icon="shopping-cart" {
        item "Orders" to="/sales/orders"
        item "Customers" to="/sales/customers"
    }
    group "Inventory" {
        item "Products" to="/inventory/products"
    }
}
```

### 5.6 Roles and Permissions (ACL)
Roles grant access to specific permissions defined in `routes` or custom actions.

```kdl
// Role definition
role "Admin" {
    allow "*" // Access everything
}

role "InventoryStaff" {
    allow "inventory.view"
    // can see list, but cannot 'inventory.create' or 'inventory.edit'
}

role "SalesManager" {
    allow "sales.*"     // all sales permissions
    allow "inventory.view" // read-only inventory
}

// User assignment (usually would be in DB, but can be in DSL for initial setup)
user "admin@gurih.erp" {
    role "Admin"
}
```

#### How it works:
1.  **Middleware level**: When a user hits `/inventory/products/new`, the system checks if the user has `inventory.create`.
2.  **UI level**: The sidebar menu will automatically hide items if the user doesn't have the permission required by the route.
3.  **Component level**: Buttons inside a page (like "Delete") can also check for specific permissions.


### 5.7 Print Format
Defines how a document looks when printed or exported to PDF.

```kdl
print "InvoiceStandard" for="Invoice" {
    title "Faktur Penjualan"
    
    header {
        left "logo"
        right "company_address"
    }
    
    section "Details" {
        field "invoice_number"
        field "date"
        field "customer.name"
    }
    
    table "items" {
        column "item_name"
        column "qty"
        column "amount"
    }
    
    footer {
        text "Thank you for your business."
    }
}
```

## 6. Example: Full Sales Module with UI

```kdl
module "Sales" {
    entity "Customer" {
        field:pk id
        field:name "name"
        field:email "email"
        field:phone "phone"
        field:text "address"
        
        has_many "orders" "Order"
    }
    
    entity "Order" {
        field:pk id
        field:serial "order_number" serial_generator="OrderCode"
        field:date "order_date"
        belongs_to "Customer"
        
        field:money "total_amount"
        field:enum "status" "OrderStatus"
    }

    page "CustomerList" {
        datatable for="Customer" {
            column "name"
            column "email"
        }
    }

    page "CustomerForm" {
        form for="Customer" {
            field "name"
            field "email"
            field "phone"
            field "address"
        }
    }
}
```

## 7. Perspective Query
The Perspective Query component allows for querying hierarchical nested data from the database, inspired by expressive query construction approaches. It combines the power of SQL with the simplicity of direct nested data manipulation.

### 7.1 Nested Query
Nested queries allow you to structure your data hierarchically, ideal for APIs or complex data views that require children records to be embedded.

```kdl
query:nested "ActiveCourseQuery" for="CourseEntity" {
    select "title"
    formula "total_duration" "SUM([duration])"
    
    join "SectionEntity" {
        select "type" 
        select "num" 
        
        join "MeetingEntity" {
            select "day"
            select "start"
            select "end"
            formula "duration" "[end] - [start]"
            formula "percent"  "ROUND([duration] / [total_duration]) * 100"
        }
    }
}
```

### 7.2 Flat Query
Flat queries are designed to produce tabular results, similar to traditional SQL. They support filtering and field aliasing, making them perfect for populating datatables requiring pagination and sorting.

```kdl
query:flat "BookQuery" for="BookEntity" {
    join "PeopleEntity" {
       select "name" as="author"
    }
    filter "[published_at] < DATE('2000-01-01')"

    select "title"
    select "price"
}
```

### 7.3 Formula Expressions
Both query types support powerful formula expressions for calculating derived values.
- **Arithmetic**: `+`, `-`, `*`, `/` provided (e.g., `[end] - [start]`).
- **Field References**: Use square brackets `[field_name]` to refer to columns.
- **Functions**: Support standard functions like `SUM()`, `AVG()`, `COUNT()`, `ROUND()`, `DATE()`, etc.

### 7.4 Usage in Datatable
You can bind a `datatable` to a defined query instead of a raw entity.

```kdl
page "CourseReport" {
    datatable query="ActiveCourseQuery" {
        column "title" label="Course Title"
        column "day"
        column "start"
        column "end"
        column "percent"
    }
}
```

When using `query`, the `for` attribute on `datatable` is optional, as the root entity is derived from the query definition.

