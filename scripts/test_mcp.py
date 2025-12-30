import subprocess
import json
import time
import sys
import os

def main():
    # Ensure CWD is correct for the relative path to script
    cwd = "/home/olivier"
    cmd = ["./Projects/buckets/.linear_mcp.sh"]
    
    print(f"Starting {cmd} in {cwd}", file=sys.stderr)
    
    process = subprocess.Popen(
        cmd,
        cwd=cwd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=sys.stderr,
        text=True,
        bufsize=1
    )

    # 1. Initialize
    init_msg = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05", # Updated version
            "capabilities": {
                "roots": {"listChanged": True},
                "sampling": {}
            },
            "clientInfo": {"name": "ManualTest", "version": "1.0"}
        }
    }
    
    print(f"Sending Init...", file=sys.stderr)
    process.stdin.write(json.dumps(init_msg) + "\n")
    process.stdin.flush()
    
    # Read response
    while True:
        line = process.stdout.readline()
        if not line:
            print("Process ended unexpectedly", file=sys.stderr)
            return
            
        # Try to parse as JSON
        try:
            # Skip empty lines
            if not line.strip(): continue
            
            msg = json.loads(line)
            if msg.get("id") == 1:
                print("Received Init Response", file=sys.stderr)
                break
        except json.JSONDecodeError:
            print(f"Non-JSON output: {line.strip()}", file=sys.stderr)
            continue

    # 2. Initialized notification
    notify_msg = {
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    }
    process.stdin.write(json.dumps(notify_msg) + "\n")
    process.stdin.flush()
    
    time.sleep(0.5)

    # 3. List tools
    list_msg = {
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    }
    print(f"Sending Tools List...", file=sys.stderr)
    process.stdin.write(json.dumps(list_msg) + "\n")
    process.stdin.flush()
    
    # Read tool list
    while True:
        line = process.stdout.readline()
        if not line: break
        try:
            msg = json.loads(line)
            if msg.get("id") == 2:
                print(json.dumps(msg, indent=2)) # Print result to stdout for me to capture
                break
        except json.JSONDecodeError:
             pass

    process.terminate()

if __name__ == "__main__":
    main()
