import json
import sys

def validate(file_path):
    try:
        with open(file_path, 'r') as f:
            data = json.load(f)
        # Rudimentary structure check since we lack jsonschema lib in this env
        required = ['cursor', 'last_ok']
        for r in required:
            if r not in data:
                print(f"Missing required field: {r}")
                sys.exit(1)

        # Cursor must be integer (not null, as per new strict requirement)
        cursor = data.get('cursor')
        if not isinstance(cursor, int):
             print(f"Cursor must be int, got {type(cursor)}")
             sys.exit(1)

        # last_ok can be string or null
        last_ok = data.get('last_ok')
        if last_ok is not None and not isinstance(last_ok, str):
             print(f"last_ok must be string or null, got {type(last_ok)}")
             sys.exit(1)

        print("Valid JSON structure")
    except Exception as e:
        print(f"Validation failed: {e}")
        sys.exit(1)

if __name__ == "__main__":
    validate(sys.argv[1])
