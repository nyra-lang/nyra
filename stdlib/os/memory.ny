extern fn hw_mem_page_size() -> i32
extern fn hw_mem_map_anonymous(nbytes: i64) -> ptr
extern fn hw_mem_unmap(addr: ptr, nbytes: i64) -> i32
extern fn hw_dma_available() -> i32

// MAP-style protection flags (for documentation / future mmap wrappers).
const MEM_PROT_READ = 1
const MEM_PROT_WRITE = 2
const MEM_PROT_EXEC = 4

fn hw_page_size() -> i32 {
    return hw_mem_page_size()
}

// Anonymous RW mapping via mmap (Unix) or VirtualAlloc (Windows). Returns null ptr on failure.
fn mem_map_anonymous(nbytes: i64) -> ptr {
    return hw_mem_map_anonymous(nbytes)
}

fn mem_unmap(addr: ptr, nbytes: i64) -> i32 {
    return hw_mem_unmap(addr, nbytes)
}

extern fn hw_mem_map_file(path: string, nbytes: i64, writable: i32) -> ptr
extern fn hw_mem_sync(addr: ptr, nbytes: i64) -> i32

// File-backed mmap (MAP_SHARED). `writable=1` for read-write, `0` for read-only.
fn mem_map_file(path: string, nbytes: i64, writable: i32) -> ptr {
    return hw_mem_map_file(path, nbytes, writable)
}

fn mem_sync(addr: ptr, nbytes: i64) -> i32 {
    return hw_mem_sync(addr, nbytes)
}

// True DMA needs kernel drivers; returns false in normal userspace.
fn dma_available() -> bool {
    return hw_dma_available() == 1
}
