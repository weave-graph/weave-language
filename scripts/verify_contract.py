#!/usr/bin/env python3
"""Verify the vendored protocol bytes without network access or dependencies."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
manifest = json.loads((root / "vendor/manifest.json").read_text())
for relative, expected in manifest["files"].items():
    actual = hashlib.sha256((root / "vendor/weave-contract" / relative).read_bytes()).hexdigest()
    if actual != expected:
        raise SystemExit(f"Contract hash mismatch: {relative}")
print(f"Verified contract {manifest['contract_version']}: {len(manifest['files'])} files")
