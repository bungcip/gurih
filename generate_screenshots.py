import sys
from playwright.sync_api import sync_playwright

def run(module):
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        # Use a large viewport for better screenshots
        context = browser.new_context(viewport={'width': 1280, 'height': 800})
        page = context.new_page()

        base_url = "http://localhost:3000"

        # Inject fake user to bypass login
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
                # Using generic entity route as App.vue restricts routing
                page.goto(f"{base_url}/#/app/Account")
                page.wait_for_selector("table", timeout=10000)
                page.wait_for_timeout(1000)
                page.screenshot(path="docs/images/finance-coa-list.png")

                print("Navigating to Journal Entry Form...")
                page.goto(f"{base_url}/#/app/JournalEntry/new")
                page.wait_for_selector("form", timeout=10000)
                page.wait_for_timeout(1000)
                page.screenshot(path="docs/images/finance-journal-list.png")

            elif module == "siasn":
                print("Capturing SIASN Dashboard...")
                page.screenshot(path="docs/images/siasn-dashboard.png")

                print("Navigating to Pegawai List...")
                page.goto(f"{base_url}/#/app/Pegawai")
                page.wait_for_selector("table", timeout=10000)
                page.wait_for_timeout(1000)
                page.screenshot(path="docs/images/siasn-pegawai-list.png")

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
