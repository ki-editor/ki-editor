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
                "title": "ReceivedContext",
                "content": json.dumps(parsed_stdin_json)
            }
        }
    ]
}

# Dump to stdout
print(json.dumps(output,indent=4))