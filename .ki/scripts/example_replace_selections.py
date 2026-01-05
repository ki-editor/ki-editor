#!/usr/bin/env python3

import json

print(json.dumps({
    "dispatches": [
        {
            "ReplaceSelections": ["Coming from Python script"]
        }
    ]
}))