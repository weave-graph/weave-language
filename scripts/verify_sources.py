#!/usr/bin/env python3
"""Check original white-paper artifacts against their recorded SHA256 values."""
import hashlib
import json
from pathlib import Path

source = Path(__file__).resolve().parents[1] / "docs/source"
manifest = json.loads((source / "provenance.json").read_text())
for name, expected in manifest["files"].items():
    actual = hashlib.sha256((source / name).read_bytes()).hexdigest()
    if actual != expected:
        raise SystemExit(f"Original-paper hash mismatch: {name}")
print(f"Verified {len(manifest['files'])} original white-paper artifacts")
