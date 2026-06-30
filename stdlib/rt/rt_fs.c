#include <dirent.h>
#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

static long long copy_file(const char *src, const char *dst);
static int copy_dir_contents_inner(const char *src, const char *dst);
static int remove_dir_all_inner(const char *path);

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
    struct stat st;
    if (!path) {
        return 0;
    }
    return stat(path, &st) == 0 ? 1 : 0;
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
}

int remove_file(const char *path) {
    if (!path) {
        return -1;
    }
    return unlink(path) == 0 ? 0 : -1;
}

int create_dir(const char *path) {
    if (!path) {
        return -1;
    }
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
    while (len > 1 && buf[len - 1] == '/') {
        buf[--len] = '\0';
    }
    for (char *p = buf + 1; *p; p++) {
        if (*p == '/') {
            *p = '\0';
            if (create_dir(buf) != 0) {
                free(buf);
                return -1;
            }
            *p = '/';
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
    int need_slash = blen > 0 && base[blen - 1] != '/';
    int n = snprintf(out, out_sz, need_slash ? "%s/%s" : "%s%s", base, name);
    return (n < 0 || (size_t)n >= out_sz) ? -1 : 0;
}

static int remove_dir_all_inner(const char *path) {
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

int remove_dir_all(const char *path) {
    if (!path) {
        return -1;
    }
    struct stat st;
    if (stat(path, &st) != 0) {
        return -1;
    }
    if (!S_ISDIR(st.st_mode)) {
        return unlink(path) == 0 ? 0 : -1;
    }
    return remove_dir_all_inner(path);
}

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

int copy_dir_contents(const char *src, const char *dst) {
    if (!src || !dst) {
        return -1;
    }
    struct stat st;
    if (stat(src, &st) != 0 || !S_ISDIR(st.st_mode)) {
        return -1;
    }
    if (create_dir_all(dst) != 0) {
        return -1;
    }
    return copy_dir_contents_inner(src, dst);
}

int copy_dir(const char *src, const char *dst) {
    if (!src || !dst) {
        return -1;
    }
    struct stat st;
    if (stat(src, &st) != 0 || !S_ISDIR(st.st_mode)) {
        return -1;
    }
    if (create_dir_all(dst) != 0) {
        return -1;
    }
    return copy_dir_contents_inner(src, dst);
}

int remove_dir(const char *path) {
    if (!path) {
        return -1;
    }
    return rmdir(path) == 0 ? 0 : -1;
}

long long file_size(const char *path) {
    struct stat st;
    if (!path || stat(path, &st) != 0) {
        return -1;
    }
    return (long long)st.st_size;
}

int path_is_dir(const char *path) {
    struct stat st;
    if (!path || stat(path, &st) != 0) {
        return 0;
    }
    return S_ISDIR(st.st_mode) ? 1 : 0;
}

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
