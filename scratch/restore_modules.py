import sys
import os
import re

def extract_patch(patch_file):
    with open(patch_file, 'r') as f:
        lines = f.readlines()

    current_file = None
    file_content = []
    is_deleted = False

    for line in lines:
        if line.startswith('diff --git'):
            if current_file and is_deleted:
                save_file(current_file, file_content)
            
            # New file
            match = re.search(r'a/(.*) b/.*', line)
            if match:
                current_file = match.group(1)
            file_content = []
            is_deleted = False
        elif line.startswith('--- a/'):
            pass
        elif line.startswith('+++ /dev/null'):
            is_deleted = True
        elif line.startswith('+++ b/'):
            is_deleted = False
        elif line.startswith('@@'):
            pass
        elif line.startswith('-') and is_deleted:
            file_content.append(line[1:])
        elif line.startswith('+') and not is_deleted:
            # We don't care about added files for restoration of deleted ones
            pass
        elif line.startswith(' '):
            if is_deleted:
                file_content.append(line[1:])

    if current_file and is_deleted:
        save_file(current_file, file_content)

def save_file(path, content):
    if not path.startswith('modules/'):
        return
    
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.writelines(content)
    print(f"Restored {path}")

if __name__ == "__main__":
    extract_patch('full_diff.patch')
