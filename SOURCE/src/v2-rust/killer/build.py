#!/usr/bin/env python3
import subprocess
import os

os.chdir(r'c:\Users\skathera\Downloads\killer\native\killer_vm')

print("Building Killer with dual-syntax support...")
result = subprocess.run(
    [r'C:\Users\skathera\.cargo\bin\cargo.exe', 'build', '--release'],
    capture_output=True,
    text=True,
    timeout=300
)

print("STDOUT (last 30 lines):")
lines = result.stdout.split('\n')
for line in lines[-30:]:
    print(line)

print("\nSTDERR (last 20 lines):")
lines = result.stderr.split('\n')
for line in lines[-20:]:
    print(line)

print(f"\nExit code: {result.returncode}")

if result.returncode == 0:
    print("\n✅ Build successful!")
else:
    print("\n❌ Build failed")
