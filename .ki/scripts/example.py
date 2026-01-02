#!/usr/bin/env python3

import json
import sys

# Read and parse JSON from stdin
stdin_data = sys.stdin.read()
parsed_stdin_json = json.loads(stdin_data)

# Create the output structure
output = {
    "dispatches": [
        {
            "ShowInfo": {
                "title": "Output from example.py",
                "content": f"The received context is:\n\n{json.dumps(parsed_stdin_json,indent=4)}"
            }
        }
    ]
}

# Dump to stdout
print(json.dumps(output))