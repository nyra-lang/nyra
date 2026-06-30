# OS APIs library
# Nyra libraries

Optional Nyra library packages that build on the compiler-shipped stdlib.

## `libraries/os/`

Higher-level OS examples and platform notes. **Core APIs live in `stdlib/os/`** — import from your project:

```ny
import "../../../stdlib/os.ny"
```

See `libraries/os/README.md` and `examples/os/`.

Ready-to-use OS wrappers ship in **`stdlib/os/`** (linked automatically when you call the APIs).

| Module | Import | Purpose |
|--------|--------|---------|
| Aggregator | `import "stdlib/os.ny"` | All OS helpers |
| Syscalls | `stdlib/os/syscall.ny` | Raw `nyra_sys_*` FFI |
| POSIX-style | `stdlib/os/unistd.ny` | `os_read`, `os_write`, `os_getpid`, `os_syscall6` |
| Platform | `stdlib/os/platform.ny` | `is_linux()`, `is_darwin()`, `page_size()` |
| Environment | `stdlib/os/env.ny` | `os_getenv("HOME")` |
| Battery | `stdlib/os/battery.ny` | `battery_percent()` (macOS/Linux/Windows) |
| **CPU** | `stdlib/os/cpu.ny` | Cores, cache line, SIMD/AVX, brand string |
| **Memory** | `stdlib/os/memory.ny` | `mem_map_anonymous`, `mem_unmap`, `hw_page_size`, DMA probe |
| **Storage** | `stdlib/os/storage.ny` | Disk free/total, filesystem type |
| **Netif** | `stdlib/os/netif.ny` | Interface count, name, MAC, link up |
| **Display** | `stdlib/os/display.ny` | Width, height, refresh Hz, brightness |
| **Power** | `stdlib/os/power.ny` | AC vs battery, CPU temp (Linux), wraps `battery_percent()` |
| **Affinity** | `stdlib/os/affinity.ny` | Pin current thread to a CPU core |
| **Clocks** | `stdlib/os/clocks.ny` | `clock_rdtsc()`, `clock_monotonic_ns()` (`i64`) |
| **USB** | `stdlib/os/usb.ny` | Enumerate VID/PID (Linux sysfs) |
| **Serial** | `stdlib/os/serial.ny` | Open/read/write UART (`/dev/tty*`, `COMn`) |
| **Signals** | `stdlib/os/signals.ny` | `signal_install` + `signal_poll` (POSIX) |
| **Mqueue** | `stdlib/os/mqueue.ny` | POSIX message queues (Linux) |
| **HW crypto** | `stdlib/os/hw_crypto.ny` | `hw_random_bytes`, Secure Enclave probe |
| **Permissions** | `stdlib/os/permissions.ny` | `perm_getuid`, `perm_chroot`, Seatbelt probe |
| Asm | `stdlib/os/asm.ny` | `cpu_nop()` + Nyra `asm "..."` in `unsafe` |
| Linux syscall #s | `stdlib/os/syscall_linux.ny` | Constants for `os_syscall6` |
| Darwin syscall #s | `stdlib/os/syscall_darwin.ny` | Constants for `os_syscall6` |
| Windows constants | `stdlib/os/syscall_windows.ny` | Nt syscall #s, VirtualAlloc flags |

Runtime C modules: **`rt_hw.c`** (topology), **`rt_os_adv.c`** (affinity, clocks, USB, serial, signals, mqueue, TRNG, permissions).

## Examples

```bash
nyra run examples/os/platform/main.ny
nyra run examples/os/battery/main.ny
nyra run examples/os/hw/main.ny
nyra run examples/os/adv/main.ny
```

Docs: [webDocs/os-hardware.html](../webDocs/os-hardware.html)

## Platform notes

| Feature | Linux | macOS | Windows |
|---------|-------|-------|---------|
| USB enumerate | sysfs | FFI / bridge | FFI / bridge |
| POSIX mqueue | yes | no | no |
| Signals poll | yes | yes | no |
| `perm_chroot` | yes (root) | no | no |
| `clock_rdtsc` | x86/ARM | x86/ARM | monotonic fallback |
