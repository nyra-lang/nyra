// POSIX shared memory (shm_open + mmap MAP_SHARED).
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

#if defined(__APPLE__) || defined(__linux__)
#include <fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>
#elif defined(_WIN32)
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <io.h>
#include <fcntl.h>
#endif

static char *shm_win_name(const char *name, char *buf, size_t bufsz) {
    if (!name || !buf || bufsz < 16) {
        return NULL;
    }
    snprintf(buf, bufsz, "Local\\nyra_shm_%s", name);
    return buf;
}

static char *shm_path_name(const char *name, char *buf, size_t bufsz) {
    if (!name || !buf || bufsz < 2) {
        return NULL;
    }
#if defined(__APPLE__)
    snprintf(buf, bufsz, "/%s", name);
#else
    snprintf(buf, bufsz, "/%s", name);
#endif
    return buf;
}

int32_t shm_create(const char *name, int64_t nbytes) {
#if defined(__APPLE__) || defined(__linux__)
    char path[256];
    if (!shm_path_name(name, path, sizeof(path)) || nbytes <= 0) {
        return -1;
    }
    shm_unlink(path);
    int fd = shm_open(path, O_CREAT | O_RDWR | O_EXCL, 0600);
    if (fd < 0) {
        fd = shm_open(path, O_CREAT | O_RDWR, 0600);
    }
    if (fd < 0) {
        return -1;
    }
    if (ftruncate(fd, (off_t)nbytes) != 0) {
        close(fd);
        shm_unlink(path);
        return -1;
    }
    return (int32_t)fd;
#elif defined(_WIN32)
    char path[256];
    if (!shm_win_name(name, path, sizeof(path)) || nbytes <= 0) {
        return -1;
    }
    uint64_t size = (uint64_t)nbytes;
    HANDLE hm = CreateFileMappingA(
        INVALID_HANDLE_VALUE,
        NULL,
        PAGE_READWRITE,
        (DWORD)(size >> 32),
        (DWORD)(size & 0xffffffffu),
        path);
    if (!hm) {
        return -1;
    }
    int fd = _open_osfhandle((intptr_t)hm, O_RDWR);
    if (fd < 0) {
        CloseHandle(hm);
        return -1;
    }
    return (int32_t)fd;
#else
    (void)name;
    (void)nbytes;
    return -1;
#endif
}

int32_t shm_open_existing(const char *name, int64_t nbytes) {
#if defined(__APPLE__) || defined(__linux__)
    char path[256];
    if (!shm_path_name(name, path, sizeof(path)) || nbytes <= 0) {
        return -1;
    }
    int fd = shm_open(path, O_RDWR, 0600);
    return fd < 0 ? -1 : (int32_t)fd;
#elif defined(_WIN32)
    char path[256];
    if (!shm_win_name(name, path, sizeof(path)) || nbytes <= 0) {
        return -1;
    }
    HANDLE hm = OpenFileMappingA(FILE_MAP_ALL_ACCESS, FALSE, path);
    if (!hm) {
        return -1;
    }
    int fd = _open_osfhandle((intptr_t)hm, O_RDWR);
    return fd < 0 ? -1 : (int32_t)fd;
#else
    (void)name;
    (void)nbytes;
    return -1;
#endif
}

void *shm_map(int32_t fd, int64_t nbytes) {
#if defined(__APPLE__) || defined(__linux__)
    if (fd < 0 || nbytes <= 0) {
        return NULL;
    }
    void *p = mmap(NULL, (size_t)nbytes, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    return p == MAP_FAILED ? NULL : p;
#elif defined(_WIN32)
    if (fd < 0 || nbytes <= 0) {
        return NULL;
    }
    HANDLE hm = (HANDLE)_get_osfhandle(fd);
    if (!hm || hm == INVALID_HANDLE_VALUE) {
        return NULL;
    }
    return MapViewOfFile(hm, FILE_MAP_ALL_ACCESS, 0, 0, (SIZE_T)nbytes);
#else
    (void)fd;
    (void)nbytes;
    return NULL;
#endif
}

int32_t shm_unmap(void *addr, int64_t nbytes) {
#if defined(__APPLE__) || defined(__linux__)
    if (!addr || nbytes <= 0) {
        return -1;
    }
    return munmap(addr, (size_t)nbytes) == 0 ? 0 : -1;
#elif defined(_WIN32)
    return addr && UnmapViewOfFile(addr) ? 0 : -1;
#else
    (void)addr;
    (void)nbytes;
    return -1;
#endif
}

int32_t shm_close_fd(int32_t fd) {
#if defined(__APPLE__) || defined(__linux__)
    return fd >= 0 && close(fd) == 0 ? 0 : -1;
#elif defined(_WIN32)
    if (fd < 0) {
        return -1;
    }
    HANDLE hm = (HANDLE)_get_osfhandle(fd);
    _close(fd);
    return hm ? (CloseHandle(hm) ? 0 : -1) : 0;
#else
    (void)fd;
    return -1;
#endif
}

int32_t shm_unlink_region(const char *name) {
#if defined(__APPLE__) || defined(__linux__)
    char path[256];
    if (!shm_path_name(name, path, sizeof(path))) {
        return -1;
    }
    return shm_unlink(path) == 0 ? 0 : -1;
#elif defined(_WIN32)
    (void)name;
    return 0;
#else
    (void)name;
    return -1;
#endif
}
