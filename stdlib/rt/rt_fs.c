#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined(_WIN32)
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <direct.h>
#include <io.h>
#include <sys/stat.h>
#ifndef PATH_MAX
#define PATH_MAX MAX_PATH
#endif
#else
#include <dirent.h>
#include <sys/stat.h>
#include <unistd.h>
#endif

static long long copy_file(const char *src, const char *dst);
static int copy_dir_contents_inner(const char *src, const char *dst);
static int remove_dir_all_inner(const char *path);

#if defined(_WIN32)
#define PATHSEP_CH '\\'
#else
#define PATHSEP_CH '/'
#endif

static char *fs_append_line(char *base, const char *line) {
    if (!line) {
        return base;
    }
    size_t blen = base ? strlen(base) : 0;
    size_t llen = strlen(line);
    char *out = (char *)realloc(base, blen + llen + 2);
    if (!out) {
        free(base);
        return strdup("");
    }
    if (blen > 0) {
        out[blen] = '\n';
        memcpy(out + blen + 1, line, llen + 1);
    } else {
        memcpy(out, line, llen + 1);
    }
    return out;
}

char *read_file(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) {
        return NULL;
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        fclose(f);
        return NULL;
    }
    long sz = ftell(f);
    if (sz < 0) {
        fclose(f);
        return NULL;
    }
    rewind(f);
    char *buf = (char *)malloc((size_t)sz + 1);
    if (!buf) {
        fclose(f);
        return NULL;
    }
    size_t n = fread(buf, 1, (size_t)sz, f);
    fclose(f);
    buf[n] = '\0';
    return buf;
}

char *read_file_limit(const char *path, int max_bytes) {
    FILE *f = fopen(path, "rb");
    if (!f) {
        return NULL;
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        fclose(f);
        return NULL;
    }
    long sz = ftell(f);
    if (sz < 0) {
        fclose(f);
        return NULL;
    }
    rewind(f);
    long to_read = sz;
    if (max_bytes > 0 && to_read > max_bytes) {
        to_read = max_bytes;
    }
    char *buf = (char *)malloc((size_t)to_read + 1);
    if (!buf) {
        fclose(f);
        return NULL;
    }
    size_t n = fread(buf, 1, (size_t)to_read, f);
    fclose(f);
    buf[n] = '\0';
    return buf;
}

int write_file(const char *path, const char *content) {
    FILE *f = fopen(path, "wb");
    if (!f) {
        return -1;
    }
    size_t len = strlen(content);
    size_t n = fwrite(content, 1, len, f);
    fclose(f);
    return (int)((n == len) ? 0 : -1);
}

int file_exists(const char *path) {
    if (!path) {
        return 0;
    }
#if defined(_WIN32)
    struct _stat st;
    return _stat(path, &st) == 0 ? 1 : 0;
#else
    struct stat st;
    return stat(path, &st) == 0 ? 1 : 0;
#endif
}

int append_file(const char *path, const char *content) {
    FILE *f = fopen(path, "ab");
    if (!f || !content) {
        return -1;
    }
    size_t len = strlen(content);
    size_t n = fwrite(content, 1, len, f);
    fclose(f);
    return (int)((n == len) ? 0 : -1);
}

int fsync_file(const char *path) {
    if (!path) {
        return -1;
    }
#if defined(_WIN32)
    HANDLE h = CreateFileA(
        path,
        GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        NULL,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        NULL);
    if (h == INVALID_HANDLE_VALUE) {
        return -1;
    }
    BOOL ok = FlushFileBuffers(h);
    CloseHandle(h);
    return ok ? 0 : -1;
#else
    FILE *f = fopen(path, "r+b");
    if (!f) {
        f = fopen(path, "wb");
    }
    if (!f) {
        return -1;
    }
    fflush(f);
#if defined(__APPLE__) || defined(__linux__)
    int fd = fileno(f);
    int rc = fsync(fd);
    fclose(f);
    return rc == 0 ? 0 : -1;
#else
    fclose(f);
    return 0;
#endif
#endif
}

int remove_file(const char *path) {
    if (!path) {
        return -1;
    }
#if defined(_WIN32)
    return _unlink(path) == 0 ? 0 : -1;
#else
    return unlink(path) == 0 ? 0 : -1;
#endif
}

int create_dir(const char *path) {
    if (!path) {
        return -1;
    }
#if defined(_WIN32)
    if (_mkdir(path) == 0) {
        return 0;
    }
    if (errno == EEXIST) {
        struct _stat st;
        if (_stat(path, &st) == 0 && (st.st_mode & _S_IFDIR)) {
            return 0;
        }
    }
    return -1;
#else
    if (mkdir(path, 0755) == 0) {
        return 0;
    }
    if (errno == EEXIST) {
        struct stat st;
        if (stat(path, &st) == 0 && S_ISDIR(st.st_mode)) {
            return 0;
        }
    }
    return -1;
#endif
}

int create_dir_all(const char *path) {
    if (!path || !*path) {
        return -1;
    }
    char *buf = strdup(path);
    if (!buf) {
        return -1;
    }
    size_t len = strlen(buf);
    while (len > 1 && (buf[len - 1] == '/' || buf[len - 1] == '\\')) {
        buf[--len] = '\0';
    }
    for (char *p = buf + 1; *p; p++) {
        if (*p == '/' || *p == '\\') {
            *p = '\0';
            if (create_dir(buf) != 0) {
                free(buf);
                return -1;
            }
            *p = PATHSEP_CH;
        }
    }
    int rc = create_dir(buf);
    free(buf);
    return rc;
}

static int fs_join_path(char *out, size_t out_sz, const char *base, const char *name) {
    if (!out || out_sz == 0 || !base || !name) {
        return -1;
    }
    size_t blen = strlen(base);
    int need_sep = blen > 0 && base[blen - 1] != '/' && base[blen - 1] != '\\';
#if defined(_WIN32)
    int n = snprintf(out, out_sz, need_sep ? "%s\\%s" : "%s%s", base, name);
#else
    int n = snprintf(out, out_sz, need_sep ? "%s/%s" : "%s%s", base, name);
#endif
    return (n < 0 || (size_t)n >= out_sz) ? -1 : 0;
}

#if defined(_WIN32)
static int remove_dir_all_inner(const char *path) {
    char pattern[PATH_MAX];
    if (snprintf(pattern, sizeof(pattern), "%s\\*", path) < 0) {
        return -1;
    }
    WIN32_FIND_DATAA fd;
    HANDLE h = FindFirstFileA(pattern, &fd);
    if (h == INVALID_HANDLE_VALUE) {
        return -1;
    }
    int rc = 0;
    do {
        const char *name = fd.cFileName;
        if (!name || strcmp(name, ".") == 0 || strcmp(name, "..") == 0) {
            continue;
        }
        char child[PATH_MAX];
        if (fs_join_path(child, sizeof(child), path, name) != 0) {
            rc = -1;
            break;
        }
        if (fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) {
            if (remove_dir_all_inner(child) != 0) {
                rc = -1;
                break;
            }
        } else if (DeleteFileA(child) == 0) {
            rc = -1;
            break;
        }
    } while (FindNextFileA(h, &fd));
    FindClose(h);
    if (rc != 0) {
        return -1;
    }
    return RemoveDirectoryA(path) ? 0 : -1;
}
#else
static int remove_dir_all_inner(const char *path) {
    struct stat st;
    if (lstat(path, &st) != 0) {
        return -1;
    }
    if (!S_ISDIR(st.st_mode)) {
        return unlink(path) == 0 ? 0 : -1;
    }
    DIR *d = opendir(path);
    if (!d) {
        return -1;
    }
    struct dirent *ent;
    while ((ent = readdir(d)) != NULL) {
        const char *name = ent->d_name;
        if (!name || strcmp(name, ".") == 0 || strcmp(name, "..") == 0) {
            continue;
        }
        char child[PATH_MAX];
        if (fs_join_path(child, sizeof(child), path, name) != 0) {
            closedir(d);
            return -1;
        }
        if (remove_dir_all_inner(child) != 0) {
            closedir(d);
            return -1;
        }
    }
    closedir(d);
    return rmdir(path) == 0 ? 0 : -1;
}
#endif

int remove_dir_all(const char *path) {
    if (!path) {
        return -1;
    }
#if defined(_WIN32)
    struct _stat st;
    if (_stat(path, &st) != 0) {
        return -1;
    }
    if (!(st.st_mode & _S_IFDIR)) {
        return _unlink(path) == 0 ? 0 : -1;
    }
#else
    struct stat st;
    if (stat(path, &st) != 0) {
        return -1;
    }
    if (!S_ISDIR(st.st_mode)) {
        return unlink(path) == 0 ? 0 : -1;
    }
#endif
    return remove_dir_all_inner(path);
}

#if defined(_WIN32)
static int copy_dir_contents_inner(const char *src, const char *dst) {
    char pattern[PATH_MAX];
    if (snprintf(pattern, sizeof(pattern), "%s\\*", src) < 0) {
        return -1;
    }
    WIN32_FIND_DATAA fd;
    HANDLE h = FindFirstFileA(pattern, &fd);
    if (h == INVALID_HANDLE_VALUE) {
        return -1;
    }
    int rc = 0;
    do {
        const char *name = fd.cFileName;
        if (!name || strcmp(name, ".") == 0 || strcmp(name, "..") == 0) {
            continue;
        }
        char from[PATH_MAX];
        char to[PATH_MAX];
        if (fs_join_path(from, sizeof(from), src, name) != 0 ||
            fs_join_path(to, sizeof(to), dst, name) != 0) {
            rc = -1;
            break;
        }
        if (fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) {
            if (create_dir_all(to) != 0 || copy_dir_contents_inner(from, to) != 0) {
                rc = -1;
                break;
            }
        } else if (copy_file(from, to) < 0) {
            rc = -1;
            break;
        }
    } while (FindNextFileA(h, &fd));
    FindClose(h);
    return rc;
}
#else
static int copy_dir_contents_inner(const char *src, const char *dst) {
    DIR *d = opendir(src);
    if (!d) {
        return -1;
    }
    struct dirent *ent;
    while ((ent = readdir(d)) != NULL) {
        const char *name = ent->d_name;
        if (!name || strcmp(name, ".") == 0 || strcmp(name, "..") == 0) {
            continue;
        }
        char from[PATH_MAX];
        char to[PATH_MAX];
        if (fs_join_path(from, sizeof(from), src, name) != 0 ||
            fs_join_path(to, sizeof(to), dst, name) != 0) {
            closedir(d);
            return -1;
        }
        struct stat st;
        if (stat(from, &st) != 0) {
            closedir(d);
            return -1;
        }
        if (S_ISDIR(st.st_mode)) {
            if (create_dir_all(to) != 0 || copy_dir_contents_inner(from, to) != 0) {
                closedir(d);
                return -1;
            }
        } else if (copy_file(from, to) < 0) {
            closedir(d);
            return -1;
        }
    }
    closedir(d);
    return 0;
}
#endif

int copy_dir_contents(const char *src, const char *dst) {
    if (!src || !dst) {
        return -1;
    }
#if defined(_WIN32)
    struct _stat st;
    if (_stat(src, &st) != 0 || !(st.st_mode & _S_IFDIR)) {
        return -1;
    }
#else
    struct stat st;
    if (stat(src, &st) != 0 || !S_ISDIR(st.st_mode)) {
        return -1;
    }
#endif
    if (create_dir_all(dst) != 0) {
        return -1;
    }
    return copy_dir_contents_inner(src, dst);
}

int copy_dir(const char *src, const char *dst) {
    if (!src || !dst) {
        return -1;
    }
#if defined(_WIN32)
    struct _stat st;
    if (_stat(src, &st) != 0 || !(st.st_mode & _S_IFDIR)) {
        return -1;
    }
#else
    struct stat st;
    if (stat(src, &st) != 0 || !S_ISDIR(st.st_mode)) {
        return -1;
    }
#endif
    if (create_dir_all(dst) != 0) {
        return -1;
    }
    return copy_dir_contents_inner(src, dst);
}

int remove_dir(const char *path) {
    if (!path) {
        return -1;
    }
#if defined(_WIN32)
    return _rmdir(path) == 0 ? 0 : -1;
#else
    return rmdir(path) == 0 ? 0 : -1;
#endif
}

long long file_size(const char *path) {
#if defined(_WIN32)
    struct _stat st;
    if (!path || _stat(path, &st) != 0) {
        return -1;
    }
    return (long long)st.st_size;
#else
    struct stat st;
    if (!path || stat(path, &st) != 0) {
        return -1;
    }
    return (long long)st.st_size;
#endif
}

int path_is_dir(const char *path) {
#if defined(_WIN32)
    struct _stat st;
    if (!path || _stat(path, &st) != 0) {
        return 0;
    }
    return (st.st_mode & _S_IFDIR) ? 1 : 0;
#else
    struct stat st;
    if (!path || stat(path, &st) != 0) {
        return 0;
    }
    return S_ISDIR(st.st_mode) ? 1 : 0;
#endif
}

#if defined(_WIN32)
char *list_dir(const char *path) {
    char pattern[PATH_MAX];
    if (!path || snprintf(pattern, sizeof(pattern), "%s\\*", path) < 0) {
        return strdup("");
    }
    WIN32_FIND_DATAA fd;
    HANDLE h = FindFirstFileA(pattern, &fd);
    if (h == INVALID_HANDLE_VALUE) {
        return strdup("");
    }
    char *out = strdup("");
    do {
        const char *name = fd.cFileName;
        if (!name || strcmp(name, ".") == 0 || strcmp(name, "..") == 0) {
            continue;
        }
        out = fs_append_line(out, name);
    } while (FindNextFileA(h, &fd));
    FindClose(h);
    return out ? out : strdup("");
}
#else
char *list_dir(const char *path) {
    DIR *d = opendir(path);
    if (!d) {
        return strdup("");
    }
    char *out = strdup("");
    struct dirent *ent;
    while ((ent = readdir(d)) != NULL) {
        const char *name = ent->d_name;
        if (!name) {
            continue;
        }
        if (name[0] == '.' && (name[1] == '\0' || (name[1] == '.' && name[2] == '\0'))) {
            continue;
        }
        out = fs_append_line(out, name);
    }
    closedir(d);
    return out ? out : strdup("");
}
#endif

long long copy_file(const char *src, const char *dst) {
    FILE *in = NULL;
    FILE *out = NULL;
    char buf[65536];
    long long total = 0;

    if (!src || !dst) {
        return -1;
    }
    in = fopen(src, "rb");
    if (!in) {
        return -1;
    }
    out = fopen(dst, "wb");
    if (!out) {
        fclose(in);
        return -1;
    }
    for (;;) {
        size_t n = fread(buf, 1, sizeof(buf), in);
        if (n == 0) {
            break;
        }
        if (fwrite(buf, 1, n, out) != n) {
            fclose(in);
            fclose(out);
            return -1;
        }
        total += (long long)n;
    }
    if (ferror(in)) {
        fclose(in);
        fclose(out);
        return -1;
    }
    fclose(in);
    fclose(out);
    return total;
}
// [contrib-dev:file_is_symlink:fs_file]
int file_is_symlink(const char * path) {
    if (!path) return 0;
    #if defined(_WIN32)
    DWORD attr = GetFileAttributesA(path);
    if (attr == INVALID_FILE_ATTRIBUTES) return 0;
    return (attr & FILE_ATTRIBUTE_REPARSE_POINT) ? 1 : 0;
    #else
    struct stat st;
    if (lstat(path, &st) != 0) return 0;
    return S_ISLNK(st.st_mode) ? 1 : 0;
    #endif
}
// [/contrib-dev:file_is_symlink:fs_file]

// [contrib-dev:file_mtime:fs_file]
long long file_mtime(const char * path) {
    #if defined(_WIN32)
    struct _stat st;
    if (!path || _stat(path, &st) != 0) return -1;
    return (long long)st.st_mtime;
    #else
    struct stat st;
    if (!path || stat(path, &st) != 0) return -1;
    return (long long)st.st_mtime;
    #endif
}
// [/contrib-dev:file_mtime:fs_file]

// [contrib-dev:path_is_file:fs_file]
int path_is_file(const char * path) {
    if (!path) return 0;
    #if defined(_WIN32)
    struct _stat st;
    if (_stat(path, &st) != 0) return 0;
    return (st.st_mode & _S_IFDIR) ? 0 : 1;
    #else
    struct stat st;
    if (stat(path, &st) != 0) return 0;
    return S_ISDIR(st.st_mode) ? 0 : 1;
    #endif
}
// [/contrib-dev:path_is_file:fs_file]

// [contrib-dev:rename_file:fs_file]
int rename_file(const char * src, const char * dst) {
    if (!src || !dst) return -1;
    #if defined(_WIN32)
    return MoveFileA(src, dst) ? 0 : -1;
    #else
    return rename(src, dst) == 0 ? 0 : -1;
    #endif
}
// [/contrib-dev:rename_file:fs_file]

