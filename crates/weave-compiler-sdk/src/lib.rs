//! Pointer-free byte transport. No caller memory is ever dereferenced.
#![deny(unsafe_code)]
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use weave_language::sdk::{MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES};

pub const INVALID_HANDLE: i32 = -1;
pub const BOUNDS: i32 = -2;
pub const BUDGET: i32 = -3;
pub const INCOMPLETE: i32 = -4;
pub const BUSY: i32 = -5;
pub const INTERNAL: i32 = -6;
pub const HANDLE_EXHAUSTED: i32 = -7;
pub const MAX_HANDLES: usize = 16;
pub const MAX_BUFFER_BYTES: usize = 64 * 1024 * 1024;

enum Buffer {
    Input { bytes: Vec<u8>, expected: usize },
    Output { bytes: Vec<u8>, ok: bool },
}
impl Buffer {
    fn charge(&self) -> usize {
        match self {
            Self::Input { bytes, .. } | Self::Output { bytes, .. } => bytes.capacity(),
        }
    }
}
#[derive(Default)]
struct Arena {
    buffers: BTreeMap<i32, Buffer>,
    next: i32,
    charged: usize,
    busy: bool,
}
impl Arena {
    fn id(&mut self) -> Result<i32, i32> {
        let id = self.next.checked_add(1).ok_or(HANDLE_EXHAUSTED)?;
        self.next = id;
        Ok(id)
    }
    fn input(&mut self, length: usize) -> i32 {
        if length > MAX_REQUEST_BYTES
            || self.buffers.len() + usize::from(self.busy) >= MAX_HANDLES
            || length > MAX_BUFFER_BYTES.saturating_sub(self.charged)
        {
            return BUDGET;
        }
        let id = match self.id() {
            Ok(id) => id,
            Err(e) => return e,
        };
        let mut bytes = Vec::new();
        if bytes.try_reserve_exact(length).is_err() {
            return BUDGET;
        }
        if bytes.capacity() > MAX_BUFFER_BYTES.saturating_sub(self.charged) {
            return BUDGET;
        }
        self.charged += bytes.capacity();
        self.buffers.insert(
            id,
            Buffer::Input {
                bytes,
                expected: length,
            },
        );
        id
    }
    fn write(&mut self, id: i32, word: u32, count: usize) -> i32 {
        let Some(Buffer::Input { bytes, expected }) = self.buffers.get_mut(&id) else {
            return INVALID_HANDLE;
        };
        if !(1..=2).contains(&count) || word > 65535 || count > expected.saturating_sub(bytes.len())
        {
            return BOUNDS;
        }
        bytes.extend_from_slice(&word.to_le_bytes()[..count]);
        0
    }
    fn drop_handle(&mut self, id: i32) -> i32 {
        let Some(buffer) = self.buffers.remove(&id) else {
            return INVALID_HANDLE;
        };
        self.charged -= buffer.charge();
        0
    }
    fn start(&mut self, id: i32) -> Result<(i32, Vec<u8>), i32> {
        if self.busy {
            return Err(BUSY);
        }
        let Some(Buffer::Input { bytes, expected }) = self.buffers.get(&id) else {
            return Err(INVALID_HANDLE);
        };
        if bytes.len() != *expected {
            return Err(INCOMPLETE);
        }
        if MAX_RESPONSE_BYTES > MAX_BUFFER_BYTES.saturating_sub(self.charged) {
            return Err(BUDGET);
        }
        let output_id = self.id()?;
        // Reservation replaces the consumed input's handle but counts its bytes until finish.
        self.busy = true;
        self.charged += MAX_RESPONSE_BYTES;
        let Some(Buffer::Input { bytes, .. }) = self.buffers.remove(&id) else {
            unreachable!()
        };
        Ok((output_id, bytes))
    }
    fn finish(
        &mut self,
        id: i32,
        input_bytes: usize,
        response: Result<weave_language::sdk::Response, ()>,
    ) -> i32 {
        self.busy = false;
        self.charged -= MAX_RESPONSE_BYTES + input_bytes;
        let response = match response {
            Ok(r) => r,
            Err(()) => return INTERNAL,
        };
        if response.bytes.capacity() > MAX_RESPONSE_BYTES {
            return BUDGET;
        }
        self.charged += response.bytes.capacity();
        self.buffers.insert(
            id,
            Buffer::Output {
                bytes: response.bytes,
                ok: response.ok,
            },
        );
        id
    }
}
fn arena() -> &'static Mutex<Arena> {
    static ARENA: OnceLock<Mutex<Arena>> = OnceLock::new();
    ARENA.get_or_init(|| Mutex::new(Arena::default()))
}
fn access(f: impl FnOnce(&mut Arena) -> i32) -> i32 {
    // No callbacks or compilation occur while holding the lock. Fail closed on poison.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match arena().lock() {
        Ok(mut arena) => f(&mut arena),
        Err(_) => INTERNAL,
    }))
    .unwrap_or(INTERNAL)
}
// `no_mangle` changes symbol export only; this ABI performs no unsafe memory operations.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn weave_compiler_abi_version() -> i32 {
    1
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn weave_compiler_input_new(length: u32) -> i32 {
    access(|a| a.input(length as usize))
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn weave_compiler_input_write(handle: u32, word: u32, count: u32) -> i32 {
    access(|a| a.write(handle as i32, word, count as usize))
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn weave_compiler_compile(input: u32) -> i32 {
    let started = match arena().lock() {
        Ok(mut a) => a.start(input as i32),
        Err(_) => return INTERNAL,
    };
    let (id, bytes) = match started {
        Ok(r) => r,
        Err(e) => return e,
    };
    let length = bytes.capacity();
    let result =
        std::panic::catch_unwind(|| weave_language::sdk::compile_response(&bytes)).map_err(|_| ());
    drop(bytes);
    access(|a| a.finish(id, length, result))
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn weave_compiler_output_len(handle: u32) -> i32 {
    access(|a| match a.buffers.get(&(handle as i32)) {
        Some(Buffer::Output { bytes, .. }) => bytes.len() as i32,
        _ => INVALID_HANDLE,
    })
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn weave_compiler_output_kind(handle: u32) -> i32 {
    access(|a| match a.buffers.get(&(handle as i32)) {
        Some(Buffer::Output { ok, .. }) => i32::from(!ok),
        _ => INVALID_HANDLE,
    })
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn weave_compiler_output_read(handle: u32, offset: u32) -> i32 {
    access(|a| match a.buffers.get(&(handle as i32)) {
        Some(Buffer::Output { bytes, .. }) => {
            let offset = offset as usize;
            if offset >= bytes.len() {
                return BOUNDS;
            }
            i32::from(bytes[offset]) | (i32::from(bytes.get(offset + 1).copied().unwrap_or(0)) << 8)
        }
        _ => INVALID_HANDLE,
    })
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn weave_compiler_drop(handle: u32) -> i32 {
    access(|a| a.drop_handle(handle as i32))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ownership_and_range_failures_preserve_input() {
        let mut a = Arena::default();
        let id = a.input(3);
        assert_eq!(a.start(id), Err(INCOMPLETE));
        assert_eq!(a.write(id, 65536, 2), BOUNDS);
        assert_eq!(a.write(id, 0, 3), BOUNDS);
        assert_eq!(a.write(id, 0x6261, 2), 0);
        assert_eq!(a.write(id, 0x63, 2), BOUNDS);
        assert_eq!(a.write(id, 0x63, 1), 0);
        let (out, bytes) = a.start(id).unwrap();
        assert_eq!(bytes, b"abc");
        assert_eq!(a.drop_handle(id), INVALID_HANDLE);
        assert_eq!(a.start(id), Err(BUSY));
        assert_eq!(a.finish(out, 3, Err(())), INTERNAL);
        assert_eq!(a.charged, 0);
        assert!(!a.busy);
        let newer = a.input(0);
        assert!(newer > out);
        assert_eq!(a.drop_handle(newer), 0);
        assert_eq!(a.drop_handle(newer), INVALID_HANDLE);
    }
    #[test]
    fn quotas_reserved_output_and_handle_exhaustion() {
        let mut a = Arena::default();
        let mut ids = Vec::new();
        for _ in 0..MAX_HANDLES {
            ids.push(a.input(0));
        }
        assert!(ids.iter().all(|i| *i > 0));
        assert_eq!(a.input(0), BUDGET);
        for id in ids {
            assert_eq!(a.drop_handle(id), 0);
        }
        for _ in 0..8 {
            assert!(a.input(MAX_REQUEST_BYTES) > 0);
        }
        assert_eq!(a.input(1), BUDGET);
        let id = a.input(0);
        assert!(id > 0);
        assert_eq!(a.start(id), Err(BUDGET));
        assert_eq!(a.drop_handle(id), 0);
        let mut a = Arena {
            next: i32::MAX,
            ..Default::default()
        };
        assert_eq!(a.input(0), HANDLE_EXHAUSTED);
        assert_eq!(a.charged, 0);
    }
    #[test]
    fn cleanup_after_panic_and_reservation_during_compile() {
        let mut a = Arena::default();
        let id = a.input(0);
        let (out, bytes) = a.start(id).unwrap();
        assert_eq!(a.charged, MAX_RESPONSE_BYTES);
        for _ in 0..MAX_HANDLES - 1 {
            assert!(a.input(0) > 0);
        }
        assert_eq!(a.input(0), BUDGET);
        let panic = std::panic::catch_unwind(|| -> weave_language::sdk::Response {
            panic!("test callback")
        })
        .map_err(|_| ());
        assert_eq!(a.finish(out, bytes.len(), panic), INTERNAL);
        assert_eq!(a.charged, 0);
        assert!(a.input(0) > 0);
    }
    #[test]
    fn actual_capacity_is_retained_and_released() {
        let mut a = Arena::default();
        let mut bytes = Vec::with_capacity(4096);
        bytes.push(b'x');
        let input_capacity = bytes.capacity();
        a.buffers.insert(1, Buffer::Input { bytes, expected: 1 });
        a.next = 1;
        a.charged = input_capacity;
        let (id, bytes) = a.start(1).unwrap();
        assert_eq!(a.charged, input_capacity + MAX_RESPONSE_BYTES);
        let response = weave_language::sdk::compile_response(&bytes);
        let output_capacity = response.bytes.capacity();
        assert_eq!(a.finish(id, input_capacity, Ok(response)), id);
        assert_eq!(a.charged, output_capacity);
        assert_eq!(a.drop_handle(id), 0);
        assert_eq!(a.charged, 0);
    }
}
