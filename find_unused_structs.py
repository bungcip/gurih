import os
import re

def find_defined_structs():
    structs = {}
    for root, _, files in os.walk('crates'):
        for f in files:
            if f.endswith('.rs'):
                path = os.path.join(root, f)
                with open(path, 'r') as file:
                    content = file.read()
                    matches = re.finditer(r'(pub )?(struct|enum|trait) (\w+)', content)
                    for match in matches:
                        name = match.group(3)
                        structs[name] = path
    return structs

def count_usage(struct_name):
    # This is rough but good enough for a heuristic
    count = 0
    for root, _, files in os.walk('crates'):
        for f in files:
            if f.endswith('.rs'):
                path = os.path.join(root, f)
                with open(path, 'r') as file:
                    content = file.read()
                    # Count occurrences of the struct name, not as part of another word
                    matches = re.findall(r'\b' + struct_name + r'\b', content)
                    count += len(matches)
    return count

structs = find_defined_structs()
unused = []
for name, path in structs.items():
    if count_usage(name) <= 1: # 1 for the definition
        unused.append((name, path))

print("Potentially unused structs:")
for name, path in unused:
    print(f"{name} in {path}")
