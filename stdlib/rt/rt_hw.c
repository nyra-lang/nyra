// Hardware topology & OS-level device queries (macOS, Linux, Windows).
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined(__APPLE__)
#include <sys/sysctl.h>
#include <sys/mount.h>
#include <sys/mman.h>
#include <fcntl.h>
#include <mach/mach.h>
#include <net/if.h>
#include <net/if_dl.h>
#include <ifaddrs.h>
#include <CoreFoundation/CoreFoundation.h>
#include <CoreGraphics/CoreGraphics.h>
#include <IOKit/IOKitLib.h>
#include <IOKit/ps/IOPowerSources.h>
#elif defined(__linux__)
#include <unistd.h>
#include <sys/sysinfo.h>
#include <sys/mman.h>
#include <sys/statvfs.h>
#include <sys/socket.h>
#include <sys/ioctl.h>
#include <fcntl.h>
#include <net/if.h>
#include <ifaddrs.h>
#include <dirent.h>
#elif defined(_WIN32)
#ifndef WINVER
#define WINVER 0x0600
#endif
#ifndef _WIN32_WINNT
#define _WIN32_WINNT 0x0600
#endif
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <winsock2.h>
#include <windows.h>
#include <iphlpapi.h>
#endif

static char *dup_cstr(const char *s) {
    if (!s) {
        char *e = (char *)malloc(1);
        if (e) {
            e[0] = '\0';
        }
        return e;
    }
    size_t n = strlen(s);
    char *out = (char *)malloc(n + 1);
    if (!out) {
        return NULL;
    }
    memcpy(out, s, n + 1);
    return out;
}

static int32_t sysctl_i32(const char *name, int32_t fallback) {
#if defined(__APPLE__)
    int32_t val = fallback;
    size_t len = sizeof(val);
    if (sysctlbyname(name, &val, &len, NULL, 0) == 0) {
        return val;
    }
    return fallback;
#else
    (void)name;
    return fallback;
#endif
}

// --- CPU ---

int32_t hw_cpu_physical_cores(void) {
#if defined(__APPLE__)
    return sysctl_i32("hw.physicalcpu", -1);
#elif defined(__linux__)
    long n = sysconf(_SC_NPROCESSORS_CONF);
    return n > 0 ? (int32_t)n : -1;
#elif defined(_WIN32)
    SYSTEM_INFO si;
    GetSystemInfo(&si);
    return (int32_t)si.dwNumberOfProcessors;
#else
    return -1;
#endif
}

int32_t hw_cpu_logical_cores(void) {
#if defined(__APPLE__)
    return sysctl_i32("hw.logicalcpu", -1);
#elif defined(__linux__)
    long n = sysconf(_SC_NPROCESSORS_ONLN);
    return n > 0 ? (int32_t)n : -1;
#elif defined(_WIN32)
    return (int32_t)GetActiveProcessorCount(ALL_PROCESSOR_GROUPS);
#else
    return -1;
#endif
}

int32_t hw_cpu_cache_line_size(void) {
#if defined(__APPLE__)
    return sysctl_i32("hw.cachelinesize", -1);
#elif defined(__linux__)
    FILE *f = fopen("/sys/devices/system/cpu/cpu0/cache/index0/coherency_line_size", "r");
    if (!f) {
        return -1;
    }
    int val = -1;
    if (fscanf(f, "%d", &val) != 1) {
        val = -1;
    }
    fclose(f);
    return (int32_t)val;
#elif defined(_WIN32)
    DWORD len = 0;
    GetLogicalProcessorInformation(NULL, &len);
    PSYSTEM_LOGICAL_PROCESSOR_INFORMATION info =
        (PSYSTEM_LOGICAL_PROCESSOR_INFORMATION)malloc(len);
    if (!info) {
        return -1;
    }
    if (!GetLogicalProcessorInformation(info, &len)) {
        free(info);
        return -1;
    }
    DWORD count = len / sizeof(SYSTEM_LOGICAL_PROCESSOR_INFORMATION);
    for (DWORD i = 0; i < count; i++) {
        if (info[i].Relationship == RelationCache && info[i].Cache.Level == 1) {
            int32_t line = (int32_t)info[i].Cache.LineSize;
            free(info);
            return line;
        }
    }
    free(info);
    return -1;
#else
    return -1;
#endif
}

int32_t hw_cpu_has_sse42(void) {
#if (defined(__x86_64__) || defined(__i386__)) && (defined(__GNUC__) || defined(__clang__))
    return __builtin_cpu_supports("sse4.2") ? 1 : 0;
#else
    return 0;
#endif
}

int32_t hw_cpu_has_avx(void) {
#if (defined(__x86_64__) || defined(__i386__)) && (defined(__GNUC__) || defined(__clang__))
    return __builtin_cpu_supports("avx") ? 1 : 0;
#else
    return 0;
#endif
}

int32_t hw_cpu_has_avx2(void) {
#if (defined(__x86_64__) || defined(__i386__)) && (defined(__GNUC__) || defined(__clang__))
    return __builtin_cpu_supports("avx2") ? 1 : 0;
#else
    return 0;
#endif
}

char *hw_cpu_brand(void) {
#if defined(__APPLE__)
    char buf[256];
    size_t len = sizeof(buf);
    if (sysctlbyname("machdep.cpu.brand_string", buf, &len, NULL, 0) == 0) {
        return dup_cstr(buf);
    }
    return dup_cstr("");
#elif defined(__linux__)
    FILE *f = fopen("/proc/cpuinfo", "r");
    if (!f) {
        return dup_cstr("");
    }
    char line[512];
    while (fgets(line, sizeof(line), f)) {
        if (strncmp(line, "model name", 10) == 0) {
            char *colon = strchr(line, ':');
            if (colon) {
                colon++;
                while (*colon == ' ' || *colon == '\t') {
                    colon++;
                }
                size_t n = strlen(colon);
                while (n > 0 && (colon[n - 1] == '\n' || colon[n - 1] == '\r')) {
                    colon[--n] = '\0';
                }
                fclose(f);
                return dup_cstr(colon);
            }
        }
    }
    fclose(f);
    return dup_cstr("");
#elif defined(_WIN32)
    int regs[4] = {0};
    char brand[49];
    memset(brand, 0, sizeof(brand));
#if defined(__GNUC__) || defined(__clang__)
    __asm__ volatile("cpuid"
                     : "=a"(regs[0]), "=b"(regs[1]), "=c"(regs[2]), "=d"(regs[3])
                     : "a"(0x80000002));
    memcpy(brand, regs, 16);
    __asm__ volatile("cpuid"
                     : "=a"(regs[0]), "=b"(regs[1]), "=c"(regs[2]), "=d"(regs[3])
                     : "a"(0x80000003));
    memcpy(brand + 16, regs, 16);
    __asm__ volatile("cpuid"
                     : "=a"(regs[0]), "=b"(regs[1]), "=c"(regs[2]), "=d"(regs[3])
                     : "a"(0x80000004));
    memcpy(brand + 32, regs, 16);
    return dup_cstr(brand);
#else
    return dup_cstr("");
#endif
#else
    return dup_cstr("");
#endif
}

// --- Memory ---

int32_t hw_mem_page_size(void) {
#if defined(_WIN32)
    SYSTEM_INFO si;
    GetSystemInfo(&si);
    return (int32_t)si.dwPageSize;
#else
    long p = sysconf(_SC_PAGESIZE);
    return p > 0 ? (int32_t)p : 4096;
#endif
}

void *hw_mem_map_anonymous(int64_t nbytes) {
    if (nbytes <= 0) {
        return NULL;
    }
#if defined(__APPLE__) || defined(__linux__)
    void *p = mmap(NULL, (size_t)nbytes, PROT_READ | PROT_WRITE,
                   MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    return p == MAP_FAILED ? NULL : p;
#elif defined(_WIN32)
    return VirtualAlloc(NULL, (SIZE_T)nbytes, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
#else
    (void)nbytes;
    return NULL;
#endif
}

int32_t hw_mem_unmap(void *addr, int64_t nbytes) {
    if (!addr || nbytes <= 0) {
        return -1;
    }
#if defined(__APPLE__) || defined(__linux__)
    return munmap(addr, (size_t)nbytes) == 0 ? 0 : -1;
#elif defined(_WIN32)
    return VirtualFree(addr, 0, MEM_RELEASE) ? 0 : -1;
#else
    (void)addr;
    (void)nbytes;
    return -1;
#endif
}

int32_t hw_dma_available(void) {
    /* True DMA requires kernel drivers; not exposed in normal userspace. */
    return 0;
}

void *hw_mem_map_file(const char *path, int64_t nbytes, int32_t writable) {
    if (!path || !path[0] || nbytes <= 0) {
        return NULL;
    }
#if defined(__APPLE__) || defined(__linux__)
    int flags = writable ? O_RDWR : O_RDONLY;
    int fd = open(path, flags);
    if (fd < 0) {
        return NULL;
    }
    int prot = PROT_READ | (writable ? PROT_WRITE : 0);
    void *p = mmap(NULL, (size_t)nbytes, prot, MAP_SHARED, fd, 0);
    close(fd);
    return p == MAP_FAILED ? NULL : p;
#elif defined(_WIN32)
    HANDLE hf = CreateFileA(path, writable ? (GENERIC_READ | GENERIC_WRITE) : GENERIC_READ,
                            FILE_SHARE_READ | FILE_SHARE_WRITE, NULL, OPEN_EXISTING,
                            FILE_ATTRIBUTE_NORMAL, NULL);
    if (hf == INVALID_HANDLE_VALUE) {
        return NULL;
    }
    HANDLE hm = CreateFileMappingA(hf, NULL, writable ? PAGE_READWRITE : PAGE_READONLY,
                                   0, 0, NULL);
    CloseHandle(hf);
    if (!hm) {
        return NULL;
    }
    void *p = MapViewOfFile(hm, writable ? FILE_MAP_WRITE : FILE_MAP_READ, 0, 0, (SIZE_T)nbytes);
    CloseHandle(hm);
    return p;
#else
    (void)writable;
    return NULL;
#endif
}

int32_t hw_mem_sync(void *addr, int64_t nbytes) {
    if (!addr || nbytes <= 0) {
        return -1;
    }
#if defined(__APPLE__) || defined(__linux__)
    return msync(addr, (size_t)nbytes, MS_SYNC) == 0 ? 0 : -1;
#else
    (void)addr;
    (void)nbytes;
    return -1;
#endif
}

// --- Storage ---

int64_t hw_disk_total_bytes(const char *path) {
    if (!path || !path[0]) {
        path = "/";
    }
#if defined(__APPLE__)
    struct statfs st;
    if (statfs(path, &st) != 0) {
        return -1;
    }
    return (int64_t)st.f_blocks * (int64_t)st.f_bsize;
#elif defined(__linux__)
    struct statvfs st;
    if (statvfs(path, &st) != 0) {
        return -1;
    }
    return (int64_t)st.f_blocks * (int64_t)st.f_frsize;
#elif defined(_WIN32)
    ULARGE_INTEGER total = {0};
    if (!GetDiskFreeSpaceExA(path, NULL, &total, NULL)) {
        return -1;
    }
    return (int64_t)total.QuadPart;
#else
    (void)path;
    return -1;
#endif
}

int64_t hw_disk_free_bytes(const char *path) {
    if (!path || !path[0]) {
        path = "/";
    }
#if defined(__APPLE__)
    struct statfs st;
    if (statfs(path, &st) != 0) {
        return -1;
    }
    return (int64_t)st.f_bavail * (int64_t)st.f_bsize;
#elif defined(__linux__)
    struct statvfs st;
    if (statvfs(path, &st) != 0) {
        return -1;
    }
    return (int64_t)st.f_bavail * (int64_t)st.f_frsize;
#elif defined(_WIN32)
    ULARGE_INTEGER free_bytes = {0};
    if (!GetDiskFreeSpaceExA(path, &free_bytes, NULL, NULL)) {
        return -1;
    }
    return (int64_t)free_bytes.QuadPart;
#else
    (void)path;
    return -1;
#endif
}

char *hw_disk_fs_type(const char *path) {
    if (!path || !path[0]) {
        path = "/";
    }
#if defined(__APPLE__)
    struct statfs st;
    if (statfs(path, &st) != 0) {
        return dup_cstr("");
    }
    return dup_cstr(st.f_fstypename);
#elif defined(__linux__)
    struct statvfs st;
    if (statvfs(path, &st) != 0) {
        return dup_cstr("");
    }
    char buf[64];
    snprintf(buf, sizeof(buf), "0x%lx", (unsigned long)st.f_type);
    return dup_cstr(buf);
#elif defined(_WIN32)
    char fs_name[MAX_PATH + 1];
    if (GetVolumeInformationA(path, NULL, 0, NULL, NULL, NULL, fs_name, MAX_PATH)) {
        return dup_cstr(fs_name);
    }
    return dup_cstr("");
#else
    (void)path;
    return dup_cstr("");
#endif
}

// --- Network interfaces ---

typedef struct {
    char name[32];
    char mac[24];
    int up;
} NyraNetIf;

#if defined(__APPLE__) || defined(__linux__)
static int collect_netifs(NyraNetIf *out, int max) {
    struct ifaddrs *ifap = NULL;
    if (getifaddrs(&ifap) != 0) {
        return 0;
    }
    int n = 0;
    for (struct ifaddrs *ifa = ifap; ifa && n < max; ifa = ifa->ifa_next) {
        if (!ifa->ifa_name) {
            continue;
        }
        int found = -1;
        for (int i = 0; i < n; i++) {
            if (strcmp(out[i].name, ifa->ifa_name) == 0) {
                found = i;
                break;
            }
        }
        int idx = found;
        if (idx < 0) {
            idx = n++;
            memset(&out[idx], 0, sizeof(out[idx]));
            strncpy(out[idx].name, ifa->ifa_name, sizeof(out[idx].name) - 1);
            out[idx].up = (ifa->ifa_flags & IFF_UP) ? 1 : 0;
        }
#if defined(__APPLE__)
        if (ifa->ifa_addr && ifa->ifa_addr->sa_family == AF_LINK) {
            struct sockaddr_dl *sdl = (struct sockaddr_dl *)ifa->ifa_addr;
            unsigned char *ptr = (unsigned char *)LLADDR(sdl);
            snprintf(out[idx].mac, sizeof(out[idx].mac),
                     "%02x:%02x:%02x:%02x:%02x:%02x", ptr[0], ptr[1], ptr[2], ptr[3],
                     ptr[4], ptr[5]);
        }
#elif defined(__linux__)
        if (out[idx].mac[0] == '\0') {
            int fd = socket(AF_INET, SOCK_DGRAM, 0);
            if (fd >= 0) {
                struct ifreq ifr;
                memset(&ifr, 0, sizeof(ifr));
                strncpy(ifr.ifr_name, ifa->ifa_name, IFNAMSIZ - 1);
                if (ioctl(fd, SIOCGIFHWADDR, &ifr) == 0) {
                    unsigned char *ptr = (unsigned char *)ifr.ifr_hwaddr.sa_data;
                    snprintf(out[idx].mac, sizeof(out[idx].mac),
                             "%02x:%02x:%02x:%02x:%02x:%02x", ptr[0], ptr[1], ptr[2],
                             ptr[3], ptr[4], ptr[5]);
                }
                close(fd);
            }
        }
#endif
    }
    freeifaddrs(ifap);
    return n;
}
#endif

int32_t hw_net_if_count(void) {
#if defined(__APPLE__) || defined(__linux__)
    NyraNetIf tmp[64];
    return collect_netifs(tmp, 64);
#elif defined(_WIN32)
    ULONG len = 0;
    if (GetAdaptersAddresses(AF_UNSPEC, 0, NULL, NULL, &len) != ERROR_BUFFER_OVERFLOW) {
        return 0;
    }
    IP_ADAPTER_ADDRESSES *addrs = (IP_ADAPTER_ADDRESSES *)malloc(len);
    if (!addrs) {
        return 0;
    }
    if (GetAdaptersAddresses(AF_UNSPEC, 0, NULL, addrs, &len) != NO_ERROR) {
        free(addrs);
        return 0;
    }
    int n = 0;
    for (IP_ADAPTER_ADDRESSES *a = addrs; a; a = a->Next) {
        if (a->OperStatus == IfOperStatusUp || a->PhysicalAddressLength > 0) {
            n++;
        }
    }
    free(addrs);
    return n;
#else
    return 0;
#endif
}

static int netif_at(int32_t index, NyraNetIf *out) {
#if defined(__APPLE__) || defined(__linux__)
    NyraNetIf tmp[64];
    int n = collect_netifs(tmp, 64);
    if (index < 0 || index >= n) {
        return -1;
    }
    *out = tmp[index];
    return 0;
#elif defined(_WIN32)
    ULONG len = 0;
    GetAdaptersAddresses(AF_UNSPEC, 0, NULL, NULL, &len);
    IP_ADAPTER_ADDRESSES *addrs = (IP_ADAPTER_ADDRESSES *)malloc(len);
    if (!addrs) {
        return -1;
    }
    if (GetAdaptersAddresses(AF_UNSPEC, 0, NULL, addrs, &len) != NO_ERROR) {
        free(addrs);
        return -1;
    }
    int cur = 0;
    for (IP_ADAPTER_ADDRESSES *a = addrs; a; a = a->Next) {
        if (!(a->OperStatus == IfOperStatusUp || a->PhysicalAddressLength > 0)) {
            continue;
        }
        if (cur == index) {
            memset(out, 0, sizeof(*out));
            WideCharToMultiByte(CP_UTF8, 0, a->FriendlyName, -1, out->name,
                                (int)sizeof(out->name), NULL, NULL);
            if (a->PhysicalAddressLength >= 6) {
                snprintf(out->mac, sizeof(out->mac), "%02x:%02x:%02x:%02x:%02x:%02x",
                         a->PhysicalAddress[0], a->PhysicalAddress[1],
                         a->PhysicalAddress[2], a->PhysicalAddress[3],
                         a->PhysicalAddress[4], a->PhysicalAddress[5]);
            }
            out->up = (a->OperStatus == IfOperStatusUp) ? 1 : 0;
            free(addrs);
            return 0;
        }
        cur++;
    }
    free(addrs);
    return -1;
#else
    (void)index;
    (void)out;
    return -1;
#endif
}

char *hw_net_if_name(int32_t index) {
    NyraNetIf iface;
    if (netif_at(index, &iface) != 0) {
        return dup_cstr("");
    }
    return dup_cstr(iface.name);
}

char *hw_net_if_mac(int32_t index) {
    NyraNetIf iface;
    if (netif_at(index, &iface) != 0) {
        return dup_cstr("");
    }
    return dup_cstr(iface.mac);
}

int32_t hw_net_if_is_up(int32_t index) {
    NyraNetIf iface;
    if (netif_at(index, &iface) != 0) {
        return -1;
    }
    return iface.up;
}

// --- Display ---

int32_t hw_display_width(void) {
#if defined(__APPLE__)
    CGRect r = CGDisplayBounds(CGMainDisplayID());
    return (int32_t)r.size.width;
#elif defined(__linux__)
    const char *dims = getenv("NYRA_DISPLAY_WIDTH");
    if (dims) {
        return (int32_t)atoi(dims);
    }
    return -1;
#elif defined(_WIN32)
    return (int32_t)GetSystemMetrics(SM_CXSCREEN);
#else
    return -1;
#endif
}

int32_t hw_display_height(void) {
#if defined(__APPLE__)
    CGRect r = CGDisplayBounds(CGMainDisplayID());
    return (int32_t)r.size.height;
#elif defined(__linux__)
    const char *dims = getenv("NYRA_DISPLAY_HEIGHT");
    if (dims) {
        return (int32_t)atoi(dims);
    }
    return -1;
#elif defined(_WIN32)
    return (int32_t)GetSystemMetrics(SM_CYSCREEN);
#else
    return -1;
#endif
}

int32_t hw_display_refresh_hz(void) {
#if defined(__APPLE__)
    CGDisplayModeRef mode = CGDisplayCopyDisplayMode(CGMainDisplayID());
    if (!mode) {
        return -1;
    }
    double hz = CGDisplayModeGetRefreshRate(mode);
    CGDisplayModeRelease(mode);
    return hz > 0 ? (int32_t)(hz + 0.5) : -1;
#elif defined(_WIN32)
    DEVMODEA dm;
    memset(&dm, 0, sizeof(dm));
    dm.dmSize = sizeof(dm);
    if (EnumDisplaySettingsA(NULL, ENUM_CURRENT_SETTINGS, &dm) && dm.dmDisplayFrequency > 0) {
        return (int32_t)dm.dmDisplayFrequency;
    }
    return -1;
#else
    return -1;
#endif
}

int32_t hw_display_brightness_pct(void) {
#if defined(__linux__)
    DIR *d = opendir("/sys/class/backlight");
    if (!d) {
        return -1;
    }
    struct dirent *ent;
    int32_t best = -1;
    while ((ent = readdir(d)) != NULL) {
        if (ent->d_name[0] == '.') {
            continue;
        }
        char maxpath[256];
        char curpath[256];
        snprintf(maxpath, sizeof(maxpath), "/sys/class/backlight/%s/max_brightness",
                 ent->d_name);
        FILE *maxf = fopen(maxpath, "r");
        if (!maxf) {
            continue;
        }
        int max_b = 0;
        fscanf(maxf, "%d", &max_b);
        fclose(maxf);
        snprintf(curpath, sizeof(curpath), "/sys/class/backlight/%s/brightness", ent->d_name);
        FILE *curf = fopen(curpath, "r");
        if (!curf || max_b <= 0) {
            if (curf) {
                fclose(curf);
            }
            continue;
        }
        int cur = 0;
        fscanf(curf, "%d", &cur);
        fclose(curf);
        int32_t pct = (int32_t)((cur * 100) / max_b);
        if (pct > best) {
            best = pct;
        }
    }
    closedir(d);
    return best;
#elif defined(_WIN32)
    return -1;
#elif defined(__APPLE__)
    return -1;
#else
    return -1;
#endif
}

// --- Power ---

int32_t hw_power_on_ac(void) {
#if defined(__APPLE__)
    CFDictionaryRef blob = IOPSCopyPowerSourcesInfo();
    if (!blob) {
        return -1;
    }
    CFArrayRef sources = IOPSCopyPowerSourcesList(blob);
    if (!sources) {
        CFRelease(blob);
        return -1;
    }
    int32_t on_ac = -1;
    CFIndex count = CFArrayGetCount(sources);
    for (CFIndex i = 0; i < count; i++) {
        CFDictionaryRef ps =
            IOPSGetPowerSourceDescription(blob, CFArrayGetValueAtIndex(sources, i));
        if (!ps) {
            continue;
        }
        CFStringRef state = (CFStringRef)CFDictionaryGetValue(ps, CFSTR("Power Source State"));
        if (!state) {
            continue;
        }
        if (CFStringCompare(state, CFSTR("AC Power"), 0) == kCFCompareEqualTo) {
            on_ac = 1;
            break;
        }
        if (CFStringCompare(state, CFSTR("Battery Power"), 0) == kCFCompareEqualTo) {
            on_ac = 0;
        }
    }
    CFRelease(sources);
    CFRelease(blob);
    return on_ac;
#elif defined(__linux__)
    FILE *f = fopen("/sys/class/power_supply/AC/online", "r");
    if (!f) {
        f = fopen("/sys/class/power_supply/AC0/online", "r");
    }
    if (!f) {
        return -1;
    }
    int val = -1;
    fscanf(f, "%d", &val);
    fclose(f);
    return (int32_t)val;
#elif defined(_WIN32)
    SYSTEM_POWER_STATUS st;
    if (!GetSystemPowerStatus(&st)) {
        return -1;
    }
    if (st.ACLineStatus == 1) {
        return 1;
    }
    if (st.ACLineStatus == 0) {
        return 0;
    }
    return -1;
#else
    return -1;
#endif
}

int32_t hw_power_cpu_temp_centi_c(void) {
#if defined(__linux__)
    FILE *f = fopen("/sys/class/thermal/thermal_zone0/temp", "r");
    if (!f) {
        return -1;
    }
    int milli = -1;
    if (fscanf(f, "%d", &milli) == 1) {
        fclose(f);
        return milli / 10;
    }
    fclose(f);
    return -1;
#else
    return -1;
#endif
}
