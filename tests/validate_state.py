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

        # Cursor must be integer or null
        cursor = data.get('cursor')
        if cursor is not None and not isinstance(cursor, int):
             print(f"Cursor must be int or null, got {type(cursor)}")
             sys.exit(1)

        print("Valid JSON structure")
    except Exception as e:
        print(f"Validation failed: {e}")
        sys.exit(1)

if __name__ == "__main__":
    validate(sys.argv[1])
