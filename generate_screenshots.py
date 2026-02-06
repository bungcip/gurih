import sys
from playwright.sync_api import sync_playwright
import json
import os

def render_dsl_screenshot(page, file_path, output_path, title):
    try:
        with open(file_path, 'r') as f:
            content = f.read()
            # Escape HTML characters
            content = content.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
    except FileNotFoundError:
        print(f"File not found: {file_path}")
        return

    html = f"""
    <!DOCTYPE html>
    <html>
    <head>
        <style>
            body {{ margin: 0; background: #1e1e1e; font-family: 'Menlo', 'Monaco', 'Courier New', monospace; color: #d4d4d4; display: flex; justify-content: center; align-items: center; height: 100vh; }}
            .window {{ width: 900px; height: 600px; display: flex; flex-direction: column; background: #1e1e1e; box-shadow: 0 10px 30px rgba(0,0,0,0.5); border-radius: 6px; overflow: hidden; }}
            .titlebar {{ background: #3c3c3c; height: 30px; display: flex; align-items: center; justify-content: center; color: #cccccc; font-size: 12px; position: relative; }}
            .controls {{ position: absolute; left: 10px; display: flex; gap: 6px; }}
            .dot {{ width: 12px; height: 12px; border-radius: 50%; }}
            .red {{ background: #ff5f56; }}
            .yellow {{ background: #ffbd2e; }}
            .green {{ background: #27c93f; }}
            .content {{ flex: 1; padding: 20px; overflow: auto; white-space: pre; font-size: 13px; line-height: 1.5; color: #d4d4d4; }}
            .line-numbers {{ color: #858585; margin-right: 15px; text-align: right; user-select: none; }}
        </style>
    </head>
    <body>
        <div class="window">
            <div class="titlebar">
                <div class="controls">
                    <div class="dot red"></div>
                    <div class="dot yellow"></div>
                    <div class="dot green"></div>
                </div>
                {title}
            </div>
            <div class="content">{content}</div>
        </div>
    </body>
    </html>
    """
    page.set_content(html)
    page.locator(".window").screenshot(path=output_path)
    print(f"Captured DSL screenshot: {output_path}")

def get_mocks(module):
    mocks = []

    # Common Menu
    mocks.append({
        "url": "**/api/ui/portal",
        "json": [
            {
                "label": "Finance",
                "items": [
                    {"label": "Chart of Accounts", "entity": "Account"},
                    {"label": "Journal Entries", "entity": "JournalEntry"}
                ]
            },
            {
                "label": "Kepegawaian",
                "items": [
                    {"label": "Data Pegawai", "entity": "Pegawai"}
                ]
            }
        ]
    })

    # Dashboard
    mocks.append({
        "url": "**/api/ui/dashboard/HRDashboard",
        "json": {
            "layout": "Grid",
            "widgets": [
                {"type": "stat", "label": "Total ASN", "value": "150", "icon": "users"},
                {"type": "stat", "label": "Cuti Pending", "value": "5", "icon": "clock", "color": "warning"},
                {"type": "pie", "label": "Status", "value": [{"label": "PNS", "value": 100}, {"label": "CPNS", "value": 50}]}
            ]
        }
    })

    if module == "finance":
        # Account List
        mocks.append({
            "url": "**/api/ui/page/Account",
            "json": {
                "title": "Chart of Accounts",
                "layout": "TableView",
                "entity": "Account",
                "columns": [
                    {"field": "code", "label": "Code"},
                    {"field": "name", "label": "Name"},
                    {"field": "type", "label": "Type"},
                    {"field": "normal_balance", "label": "Normal Balance"}
                ],
                "actions": [
                    {"label": "Edit", "icon": "pencil", "to": "/finance/coa/:id"}
                ]
            }
        })
        mocks.append({
            "url": "**/api/Account",
            "json": [
                {"id": 1, "code": "101", "name": "Cash", "type": "Asset", "normal_balance": "Debit"},
                {"id": 2, "code": "102", "name": "Accounts Receivable", "type": "Asset", "normal_balance": "Debit"},
                {"id": 3, "code": "201", "name": "Accounts Payable", "type": "Liability", "normal_balance": "Credit"},
                {"id": 4, "code": "300", "name": "Retained Earnings", "type": "Equity", "normal_balance": "Credit"},
                {"id": 5, "code": "401", "name": "Sales Revenue", "type": "Revenue", "normal_balance": "Credit"}
            ]
        })

        # Journal List
        mocks.append({
            "url": "**/api/ui/page/JournalEntry",
            "json": {
                "title": "Journal Entries",
                "layout": "TableView",
                "entity": "JournalEntry",
                "columns": [
                    {"field": "entry_number", "label": "Entry #"},
                    {"field": "date", "label": "Date"},
                    {"field": "description", "label": "Description"},
                    {"field": "status", "label": "Status"}
                ],
                "actions": [
                    {"label": "View", "icon": "book", "to": "/finance/journals/:id"}
                ]
            }
        })
        mocks.append({
            "url": "**/api/JournalEntry",
            "json": [
                {"id": 1, "entry_number": "JE/2026/01/0001", "date": "2026-01-01", "description": "Opening Balance", "status": "Posted"},
                {"id": 2, "entry_number": "JE/2026/01/0002", "date": "2026-01-05", "description": "Office Supplies", "status": "Draft"}
            ]
        })

        # Journal Form
        mocks.append({
             "url": "**/api/ui/page/JournalEntry/new",
             "json": {
                 "title": "New Journal Entry",
                 "layout": "FormView", # Assuming FormView or similar
                 "entity": "JournalEntry",
                 "fields": [
                     {"name": "entry_number", "label": "Entry Number", "type": "text"},
                     {"name": "date", "label": "Date", "type": "date"},
                     {"name": "description", "label": "Description", "type": "text"}
                 ]
             }
        })

    elif module == "siasn":
        # Pegawai List
        mocks.append({
            "url": "**/api/ui/page/Pegawai",
            "json": {
                "title": "Daftar Pegawai",
                "layout": "TableView",
                "entity": "Pegawai",
                "columns": [
                    {"field": "nip", "label": "NIP"},
                    {"field": "nama", "label": "Nama"},
                    {"field": "status_pegawai", "label": "Status"},
                    {"field": "jabatan_nama", "label": "Jabatan"}
                ],
                "actions": [
                    {"label": "Edit", "icon": "pencil", "to": "/kepegawaian/pegawai/:id"}
                ]
            }
        })
        mocks.append({
            "url": "**/api/Pegawai",
            "json": [
                {"id": 1, "nip": "198501012010011001", "nama": "Budi Santoso", "status_pegawai": "PNS", "jabatan_nama": "Pranata Komputer Ahli Muda"},
                {"id": 2, "nip": "199205052021122002", "nama": "Siti Aminah", "status_pegawai": "CPNS", "jabatan_nama": "Analis SDM Aparatur"}
            ]
        })

    return mocks

def run(module):
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context(viewport={'width': 1280, 'height': 800})

        # Setup mocks
        mocks = get_mocks(module)

        page = context.new_page()

        # Apply mocks
        for mock in mocks:
            # Create a closure to capture json_data
            def create_handler(data):
                def handler(route, request):
                    route.fulfill(status=200, content_type="application/json", body=json.dumps(data))
                return handler

            page.route(mock["url"], create_handler(mock["json"]))

        base_url = "http://localhost:5173"

        print("Injecting fake user...")
        page.goto(base_url)
        page.evaluate("""() => {
            localStorage.setItem('user', JSON.stringify({
                username: 'admin',
                token: 'bypass-auth',
                roles: ['Admin']
            }));
        }""")

        print(f"Navigating to {base_url}...")
        try:
            page.goto(base_url, timeout=30000)
            # Wait for dashboard to load. Look for common elements.
            page.wait_for_selector("header", timeout=10000)

            # Additional wait to let charts/animations finish
            page.wait_for_timeout(2000)

            if module == "finance":
                print("Capturing Finance Dashboard...")
                page.screenshot(path="docs/images/finance-dashboard.png")

                print("Navigating to Chart of Accounts (Account List)...")
                page.goto(f"{base_url}/#/app/Account")
                page.wait_for_selector("table", timeout=10000)
                page.wait_for_timeout(1000)
                page.screenshot(path="docs/images/finance-coa-list.png")

                print("Navigating to Journal Entry List...")
                page.goto(f"{base_url}/#/app/JournalEntry")
                page.wait_for_selector("table", timeout=10000)
                page.wait_for_timeout(1000)
                page.screenshot(path="docs/images/finance-journal-list.png")

                print("Capturing DSL screenshot for CoA...")
                render_dsl_screenshot(page, "gurih-finance/coa.kdl", "docs/images/ide_coa.png", "coa.kdl")

            elif module == "siasn":
                print("Capturing SIASN Dashboard...")
                page.screenshot(path="docs/images/siasn-dashboard.png")

                print("Navigating to Pegawai List...")
                page.goto(f"{base_url}/#/app/Pegawai")
                page.wait_for_selector("table", timeout=10000)
                page.wait_for_timeout(1000)
                page.screenshot(path="docs/images/siasn-pegawai-list.png")

                print("Capturing DSL screenshot for Status...")
                render_dsl_screenshot(page, "gurih-siasn/status.kdl", "docs/images/ide_status.png", "status.kdl")

        except Exception as e:
            print(f"Error during execution: {e}")
            page.screenshot(path=f"docs/images/error_{module}.png")
        finally:
            browser.close()

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python3 generate_screenshots.py <module>")
        sys.exit(1)

    module = sys.argv[1]
    run(module)
