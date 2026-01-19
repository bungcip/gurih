
try:
    with open('gurih.kdl', 'rb') as f:
        content = f.read()
    
    offset = 360
    length = 4
    
    print(f"File size: {len(content)}")
    if offset < len(content):
        start = max(0, offset - 20)
        end = min(len(content), offset + 20)
        snippet = content[start:end]
        target = content[offset:offset+length]
        
        print(f"Target at {offset}: {target}")
        print(f"Snippet: {snippet}")
    else:
        print("Offset out of bounds")

except Exception as e:
    print(e)
