//! CLI local loader policy. The portable linker itself performs no filesystem access.
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    fs,
    io::{self, Read},
    path::{Component, Path},
};
use weave_language::modules::{MAX_TOTAL_BYTES, MAX_UNIT_BYTES, MAX_UNITS};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    id: String,
    revision: String,
    path: String,
}
pub struct Unit {
    pub id: String,
    pub revision: String,
    pub source: String,
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
pub fn read(path: &Path, limit: usize) -> io::Result<String> {
    let file = fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(invalid("Expected a regular file"));
    }
    let mut value = String::new();
    file.take(limit as u64 + 1).read_to_string(&mut value)?;
    if value.len() > limit {
        return Err(invalid("File exceeds byte budget"));
    }
    Ok(value)
}
pub fn load(path: &Path, entry_bytes: usize) -> io::Result<Vec<Unit>> {
    let map = fs::canonicalize(path)?;
    let root = map
        .parent()
        .ok_or_else(|| invalid("Map requires parent directory"))?;
    let entries: Vec<Entry> = serde_json::from_str(&read(&map, 65_536)?)?;
    if entries.len() > MAX_UNITS {
        return Err(invalid("At most 64 module-map entries"));
    }
    let mut ids = BTreeSet::new();
    let mut total = entry_bytes;
    let mut units = Vec::new();
    for entry in entries {
        if entry.id.is_empty()
            || entry.id.len() > 128
            || entry.revision.is_empty()
            || entry.revision.len() > 128
            || !ids.insert(entry.id.clone())
        {
            return Err(invalid("Invalid or duplicate module identity"));
        }
        let relative = Path::new(&entry.path);
        if entry.path.len() > 4096
            || relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(invalid(
                "Module paths must be normal relative paths inside map directory",
            ));
        }
        let target = fs::canonicalize(root.join(relative))?;
        if !target.starts_with(root) {
            return Err(invalid("Module symlink target escapes map directory"));
        }
        let remaining = MAX_TOTAL_BYTES
            .checked_sub(total)
            .ok_or_else(|| invalid("Aggregate module byte budget exceeded"))?;
        let source = read(&target, MAX_UNIT_BYTES.min(remaining))?;
        total += source.len();
        units.push(Unit {
            id: entry.id,
            revision: entry.revision,
            source,
        });
    }
    Ok(units)
}
