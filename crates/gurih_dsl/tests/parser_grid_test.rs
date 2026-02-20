use gurih_dsl::compiler::compile;
use gurih_ir::FormItem;

#[test]
fn test_parser_grid_columns() {
    let kdl = r#"
    entity "Order" {
        field "id" type="Pk"
        has_many "OrderItem"
    }

    entity "OrderItem" {
        field "id" type="Pk"
        field "product" type="String"
        field "quantity" type="Integer"
        belongs_to "Order"
    }

    page "OrderPage" {
        form "OrderForm" for="Order" {
            section "Items" {
                grid "items" {
                    column "product"
                    column "quantity"
                }
            }
        }
    }
    "#;

    let schema = compile(kdl, None).expect("Failed to compile");

    // The form name will be "OrderForm" (if name provided) or constructed.
    let form = schema.forms.get(&gurih_ir::Symbol::from("OrderForm")).expect("Form not found");
    let section = &form.sections[0];
    let item = &section.items[0];

    if let FormItem::Grid(grid) = item {
        if let Some(cols) = &grid.columns {
            assert_eq!(cols.len(), 2);
            assert_eq!(cols[0], gurih_ir::Symbol::from("product"));
            assert_eq!(cols[1], gurih_ir::Symbol::from("quantity"));
        } else {
            panic!("Grid columns should be parsed, but got None");
        }
    } else {
        panic!("Expected Grid item");
    }
}
