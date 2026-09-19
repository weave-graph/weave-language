#!/usr/bin/env python3
"""Verify the complete vendored protocol and its local versioned documentation."""
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
manifest = json.loads((root / "vendor/manifest.json").read_text())
vendor = root / "vendor/weave-contract"
actual_files = {str(path.relative_to(vendor)) for path in vendor.rglob("*") if path.is_file()}
expected_files = set(manifest["files"])
if actual_files != expected_files:
    raise SystemExit(f"Contract file set differs: missing {expected_files - actual_files}, unexpected {actual_files - expected_files}")
for relative, expected in manifest["files"].items():
    actual = hashlib.sha256((vendor / relative).read_bytes()).hexdigest()
    if actual != expected:
        raise SystemExit(f"Contract hash mismatch: {relative}")
version = manifest["contract_version"]
cargo = (vendor / "Cargo.toml").read_text()
rust = (vendor / "src/lib.rs").read_text()
if re.search(r'^version\s*=\s*"([^"]+)"', cargo, re.MULTILINE).group(1) != version:
    raise SystemExit("Contract package version differs from manifest")
if f'pub const VERSION: &str = "{version}";' not in rust:
    raise SystemExit("Contract protocol version differs from manifest")
if (vendor / "LICENSE").read_bytes() != (root / "LICENSE").read_bytes():
    raise SystemExit("Vendored license differs from documented MIT license")
documentation = manifest["documentation"]
doc = (root / documentation["path"]).read_bytes()
if hashlib.sha256(doc).hexdigest() != documentation["sha256"]:
    raise SystemExit("Contract documentation hash mismatch")
if version.encode() not in doc.splitlines()[0]:
    raise SystemExit("Contract documentation version differs from manifest")
print(f"Verified contract {version}: exact {len(expected_files)}-file set, license, package/protocol versions and documentation")
