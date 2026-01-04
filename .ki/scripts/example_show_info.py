#!/usr/bin/env python3

import json
import sys

# Read and parse JSON from stdin
stdin_data = sys.stdin.read()
parsed_stdin_json = json.loads(stdin_data)

selected_texts = json.dumps(list(map(lambda x:x["content"], parsed_stdin_json["selections"])))

# Create the output structure
output = {
    "dispatches": [
        {
            "ShowInfo": {
                "title": "Output from example.py",
                "content": f"The current selected texts are {selected_texts}"
            }
        }
    ]
}

# Dump to stdout
print(json.dumps(output))