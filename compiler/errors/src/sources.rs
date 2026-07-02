use std::collections::HashMap;
use std::sync::Mutex;

static SOURCES: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

/// Register in-memory source for a virtual file path (e.g. `compile_source`).
pub fn register_source(file: impl Into<String>, content: impl Into<String>) {
    if let Ok(mut guard) = SOURCES.lock() {
        let map = guard.get_or_insert_with(HashMap::new);
        map.insert(file.into(), content.into());
    }
}

/// Clear all registered in-memory sources.
pub fn clear_sources() {
    if let Ok(mut guard) = SOURCES.lock() {
        *guard = None;
    }
}

/// Look up source text: in-memory map first, then disk.
pub fn read_source(file: &str) -> Option<String> {
    if file.is_empty() {
        return None;
    }
    if let Ok(guard) = SOURCES.lock() {
        if let Some(map) = guard.as_ref() {
            if let Some(text) = map.get(file) {
                return Some(text.clone());
            }
        }
    }
    std::fs::read_to_string(file).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_read_in_memory_source() {
        clear_sources();
        register_source("t.ny", "fn main() {}\n");
        assert_eq!(read_source("t.ny").as_deref(), Some("fn main() {}\n"));
        clear_sources();
    }
}
