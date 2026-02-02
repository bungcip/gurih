import os
from playwright.sync_api import sync_playwright

def generate_text_screenshot(content, output_path, title=""):
    html_content = f"""
    <html>
    <head>
        <style>
            body {{ font-family: monospace; background-color: #1e1e1e; color: #d4d4d4; padding: 20px; }}
            pre {{ margin: 0; }}
            .title {{ font-weight: bold; margin-bottom: 10px; color: #569cd6; }}
        </style>
    </head>
    <body>
        <div class="title">{title}</div>
        <pre>{content}</pre>
    </body>
    </html>
    """

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page(viewport={"width": 800, "height": 600})
        page.set_content(html_content)

        # Calculate height
        bbox = page.locator("body").bounding_box()
        height = bbox["height"] + 40
        page.set_viewport_size({"width": 800, "height": int(height)})

        page.screenshot(path=output_path)
        browser.close()

def read_file_content(filepath):
    try:
        with open(filepath, 'r') as f:
            return f.read()
    except Exception as e:
        print(f"Error reading {filepath}: {e}")
        return "Error reading file"

def get_tree(path):
    # Simple tree simulation since we can't rely on 'tree' command availability or output format
    output = []
    output.append(f"{path}/")
    try:
        # Filter first to ensure is_last logic works correctly
        items = sorted([x for x in os.listdir(path) if x.endswith(".kdl")])
        for i, item in enumerate(items):
            is_last = i == len(items) - 1
            prefix = "└── " if is_last else "├── "
            output.append(f"{prefix}{item}")
    except Exception as e:
        return f"Error reading directory: {e}"
    return "\n".join(output)

def main():
    # 1. Finance Project Structure
    finance_tree = get_tree("gurih-finance")
    generate_text_screenshot(finance_tree, "docs/images/finance-project-structure.png", "Project Structure")

    # 2. Finance DSL Example (Journal)
    journal_content = read_file_content("gurih-finance/journal.kdl")

    # Extract workflow block
    lines = journal_content.splitlines()
    workflow_lines = []
    in_workflow = False
    brace_count = 0
    for line in lines:
        if "workflow \"JournalWorkflow\"" in line:
            in_workflow = True

        if in_workflow:
            workflow_lines.append(line)
            brace_count += line.count("{")
            brace_count -= line.count("}")
            if brace_count == 0:
                break

    if workflow_lines:
        dsl_content = "\n".join(workflow_lines)
    else:
        dsl_content = journal_content # Fallback

    generate_text_screenshot(dsl_content, "docs/images/finance-dsl-example.png", "gurih-finance/journal.kdl")

    # 3. Finance Integration Example
    integration_content = read_file_content("gurih-finance/integration.kdl")
    generate_text_screenshot(integration_content, "docs/images/finance-integration.png", "gurih-finance/integration.kdl")

    # 4. SIASN Project Structure
    siasn_tree = get_tree("gurih-siasn")
    generate_text_screenshot(siasn_tree, "docs/images/siasn-project-structure.png", "Project Structure")

    # 5. SIASN DSL Example (Workflow)
    workflow_content = read_file_content("gurih-siasn/workflow.kdl")

    # Extract workflow block
    lines = workflow_content.splitlines()
    status_lines = []
    in_status = False
    brace_count = 0
    for line in lines:
        if "workflow \"PegawaiStatusWorkflow\"" in line:
            in_status = True

        if in_status:
            status_lines.append(line)
            brace_count += line.count("{")
            brace_count -= line.count("}")
            if brace_count == 0:
                break

    if status_lines:
        dsl_content = "\n".join(status_lines)
    else:
        dsl_content = workflow_content

    generate_text_screenshot(dsl_content, "docs/images/siasn-dsl-example.png", "gurih-siasn/workflow.kdl")

if __name__ == "__main__":
    main()
