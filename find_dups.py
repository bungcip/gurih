import os
import re

def get_blocks(file):
    blocks = {}
    with open(file, 'r') as f:
        lines = f.readlines()

    # extract function blocks
    current_func = None
    current_lines = []

    for line in lines:
        if re.match(r'^\s*(pub )?(async )?fn ', line):
            if current_func:
                blocks[current_func] = current_lines
            current_func = line.strip()
            current_lines = [line]
        elif current_func:
            current_lines.append(line)
            if line.startswith('}'):
                blocks[current_func] = current_lines
                current_func = None

    return blocks

all_blocks = []
for root, _, files in os.walk('crates'):
    for f in files:
        if f.endswith('.rs'):
            path = os.path.join(root, f)
            blocks = get_blocks(path)
            for k, v in blocks.items():
                if len(v) > 10:
                    all_blocks.append((path, k, "".join(v)))

# check for exact duplicates
dups = {}
for path, k, v in all_blocks:
    if v not in dups:
        dups[v] = []
    dups[v].append((path, k))

for v, paths in dups.items():
    if len(paths) > 1:
        # ignore sqlite vs postgres
        is_sql = all(['sqlite' in p[0] or 'postgres' in p[0] for p in paths])
        if not is_sql:
            print(f"Duplicate found: {paths}")
