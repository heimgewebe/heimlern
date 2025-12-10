import json
import sys

try:
    values = json.loads(sys.argv[1])
except Exception:
    sys.exit(1)

for entry in values:
    if entry is None:
        continue
    print(str(entry))
