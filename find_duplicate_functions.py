import os
import re

def get_functions(file):
    functions = {}
    with open(file, 'r') as f:
        lines = f.readlines()

    current_func = None
    current_lines = []

    for line in lines:
        if re.match(r'^\s*(pub )?(async )?fn (\w+)\s*\(', line):
            if current_func:
                functions[current_func] = current_lines
            current_func = re.match(r'^\s*(pub )?(async )?fn (\w+)\s*\(', line).group(3)
            current_lines = [line]
        elif current_func:
            current_lines.append(line)
            if line.startswith('}'):
                functions[current_func] = current_lines
                current_func = None

    return functions

all_functions = []
for root, _, files in os.walk('crates'):
    for f in files:
        if f.endswith('.rs'):
            path = os.path.join(root, f)
            functions = get_functions(path)
            for k, v in functions.items():
                if len(v) > 10:
                    all_functions.append((path, k, "".join(v)))

# check for exact duplicates
dups = {}
for path, k, v in all_functions:
    # normalize whitespace to compare logic
    norm_v = re.sub(r'\s+', ' ', v)
    if norm_v not in dups:
        dups[norm_v] = []
    dups[norm_v].append((path, k))

for v, paths in dups.items():
    if len(paths) > 1:
        # ignore sqlite vs postgres and tests
        is_sql = all(['sqlite' in p[0] or 'postgres' in p[0] for p in paths])
        # is_test = all(['test' in p[0] for p in paths])
        if not is_sql:
            print(f"Duplicate found: {paths}")
