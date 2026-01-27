import subprocess
import time
import os
import signal
from playwright.sync_api import sync_playwright

def run_finance_screenshots():
    print("Starting GurihFinance...")
    # Start the server with --no-auth
    process = subprocess.Popen(
        ["./target/debug/gurih_cli", "run", "gurih-finance/gurih.kdl", "--port", "3000", "--no-auth"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        preexec_fn=os.setsid
    )

    # Wait for server to start
    print("Waiting for server to start...")
    time.sleep(15)

    try:
        with sync_playwright() as p:
            browser = p.chromium.launch(headless=True)
            context = browser.new_context(viewport={"width": 1280, "height": 800})

            # Inject fake user into localStorage to bypass frontend login screen
            context.add_init_script("""
                localStorage.setItem('user', JSON.stringify({
                    token: 'dummy-token',
                    username: 'admin',
                    roles: ['Admin'],
                    user_id: '1'
                }));
            """)

            page = context.new_page()

            # 1. Dashboard
            print("Capturing Finance Dashboard...")
            try:
                page.goto("http://localhost:3000/#/", timeout=10000)
                page.wait_for_timeout(3000) # Wait for widgets to load
                page.screenshot(path="docs/images/finance-dashboard.png")
            except Exception as e:
                print(f"Failed to capture Dashboard: {e}")

            # 2. CoA List
            print("Capturing CoA List...")
            try:
                page.goto("http://localhost:3000/#/finance/coa", timeout=10000)
                page.wait_for_timeout(2000)
                page.screenshot(path="docs/images/finance-coa-list.png")
            except Exception as e:
                 print(f"Failed to capture CoA: {e}")

            # 3. Journal List
            print("Capturing Journal List...")
            try:
                page.goto("http://localhost:3000/#/finance/journals", timeout=10000)
                page.wait_for_timeout(2000)
                page.screenshot(path="docs/images/finance-journal-list.png")
            except Exception as e:
                print(f"Failed to capture Journal: {e}")

            # 4. Reports (Trial Balance)
            print("Capturing Trial Balance...")
            try:
                page.goto("http://localhost:3000/#/finance/reports/trial-balance", timeout=10000)
                page.wait_for_timeout(2000)
                page.screenshot(path="docs/images/finance-report-trial-balance.png")
            except Exception as e:
                print(f"Failed to capture Report: {e}")

            browser.close()
    except Exception as e:
        print(f"Error capturing Finance: {e}")
    finally:
        print("Stopping GurihFinance...")
        try:
            os.killpg(os.getpgid(process.pid), signal.SIGTERM)
            process.wait()
        except Exception as e:
            print(f"Error killing process: {e}")

def run_siasn_screenshots():
    print("Starting GurihSIASN...")
    # Start the server with --no-auth
    process = subprocess.Popen(
        ["./target/debug/gurih_cli", "run", "gurih-siasn/app.kdl", "--port", "3000", "--no-auth"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        preexec_fn=os.setsid
    )

    print("Waiting for server to start...")
    time.sleep(15)

    try:
        with sync_playwright() as p:
            browser = p.chromium.launch(headless=True)
            context = browser.new_context(viewport={"width": 1280, "height": 800})

            # Inject fake user
            context.add_init_script("""
                localStorage.setItem('user', JSON.stringify({
                    token: 'dummy-token',
                    username: 'admin',
                    roles: ['Admin'],
                    user_id: '1'
                }));
            """)

            page = context.new_page()

            # 1. Dashboard
            print("Capturing SIASN Dashboard...")
            try:
                page.goto("http://localhost:3000/#/", timeout=10000)
                page.wait_for_timeout(3000)
                page.screenshot(path="docs/images/siasn-dashboard.png")
            except Exception as e:
                print(f"Failed to capture Dashboard: {e}")

            # 2. Pegawai List
            print("Capturing Pegawai List...")
            try:
                page.goto("http://localhost:3000/#/kepegawaian/pegawai", timeout=10000)
                page.wait_for_timeout(2000)
                page.screenshot(path="docs/images/siasn-pegawai-list.png")
            except Exception as e:
                print(f"Failed to capture Pegawai: {e}")

            # 3. Cuti List
            print("Capturing Cuti List...")
            try:
                page.goto("http://localhost:3000/#/cuti/pengajuan", timeout=10000)
                page.wait_for_timeout(2000)
                page.screenshot(path="docs/images/siasn-cuti-list.png")
            except Exception as e:
                print(f"Failed to capture Cuti: {e}")

            browser.close()
    except Exception as e:
        print(f"Error capturing SIASN: {e}")
    finally:
        print("Stopping GurihSIASN...")
        try:
            os.killpg(os.getpgid(process.pid), signal.SIGTERM)
            process.wait()
        except Exception as e:
             print(f"Error killing process: {e}")

if __name__ == "__main__":
    run_finance_screenshots()
    time.sleep(5)
    run_siasn_screenshots()
