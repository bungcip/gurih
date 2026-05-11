import subprocess
import json

def get_dead_code():
    res = subprocess.run(['cargo', 'clippy', '--workspace', '--message-format=json', '--', '-W', 'dead_code'], capture_output=True, text=True)
    dead_code_files = set()
    for line in res.stdout.splitlines():
        try:
            msg = json.loads(line)
            if msg.get('reason') == 'compiler-message' and msg.get('message', {}).get('code', {}).get('code') == 'dead_code':
                for span in msg['message'].get('spans', []):
                    if span.get('is_primary'):
                        print(f"Dead code found at {span['file_name']}:{span['line_start']}")
        except:
            pass

get_dead_code()
