import os
import shutil

# Files to remove from root
files_to_remove = ['interpreter.py', 'lexer.py', 'parser.py']
dirs_to_remove = ['__pycache__']

for f in files_to_remove:
    filepath = f
    if os.path.exists(filepath):
        os.remove(filepath)
        print(f"✓ Removed {f}")

for d in dirs_to_remove:
    dirpath = d
    if os.path.exists(dirpath):
        shutil.rmtree(dirpath)
        print(f"✓ Removed {d} directory")

print("\n✓ Cleanup complete!")
print("\nProject structure:")
for item in sorted(os.listdir('.')):
    if not item.startswith('.'):
        path = item
        if os.path.isdir(path):
            print(f"  📁 {item}/")
        else:
            print(f"  📄 {item}")
