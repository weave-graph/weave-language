#!/usr/bin/env python3
"""Verify the separately pinned portable geometry crate, without network access."""
import hashlib
import json
from pathlib import Path
root = Path(__file__).resolve().parents[1]
manifest = json.loads((root/'vendor/spaces-manifest.json').read_text())
vendor = root/'vendor/weave-spaces'
actual = {p.relative_to(vendor).as_posix() for p in vendor.rglob('*') if p.is_file()}
assert actual == set(manifest['files']), 'Portable spaces file set changed'
for name, expected in manifest['files'].items():
    assert hashlib.sha256((vendor/name).read_bytes()).hexdigest() == expected, name
assert (vendor/'LICENSE').read_bytes() == (root/'LICENSE').read_bytes()
print(f"Verified exact portable spaces crate: {len(actual)} files at {manifest['source_commit']}")
