# GurihERP DSL Design

This document outlines the KDL-based DSL schemas for GurihERP.

## 1. Core Structure
Every GurihERP project starts with global settings.

```kdl
name "GurihERP"
version "0.1.0"

database {
    type "postgres" // or "sqlite"
    url "env:DATABASE_URL"
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
- `id`: Primary key.
- `code`: Human-readable unique identifier (e.g., INV/2024/001). Usually linked to a `generator`.
- `sku`: Specialized code for inventory/items.
- `name`: Nama orang atau barang.
- `title`: Judul atau label pendek.
- `description`: Penjelasan panjang (textarea).
- `amount` / `money`: Nilai mata uang.
- `quantity`: Jumlah barang.
- `email` / `phone` / `address`: Data kontak.
- `status`: Keadaan (biasanya merujuk ke Enum atau Workflow).
- `timestamp` / `date`: Waktu.

### 3.3 Code Generators
Generators define rules for creating automatic human-readable codes.

```kdl
generator "InvoiceCode" {
    prefix "INV/"
    date "YYYY/"
    sequence digits=4
    // Result: INV/2024/0001
}
```

### 3.4 Entity Example with Code Grouping
```kdl
entity "Invoice" {
    id
    code "invoice_number" generator="InvoiceCode"
    date "invoice_date"
    money "total_amount"
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
    id
    // Creates a "customer" field in logic, maps to "customer_id" in DB.
    belongs_to "Customer" 
    
    // Explicit field name:
    // belongs_to "shipping_address" entity="Address"
}

entity "Customer" {
    id
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
        is_submittable true
        track_changes true
    }
}

entity "SystemSettings" {
    options {
        is_single true
    }
    string "company_name"
    string "default_currency"
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
    id
    string "item_name"
    money "amount"
}

entity "Invoice" {
    // ...
    has_many "items" "InvoiceItem" type="composition"
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
}
```

## 5. UI and Navigation

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
        id
        name
        email
        phone
        address
        
        has_many "orders" "Order"
    }
    
    entity "Order" {
        id
        code "order_number" generator="OrderCode"
        date "order_date"
        belongs_to "Customer"
        
        money "total_amount"
        enum "status" "OrderStatus"
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
