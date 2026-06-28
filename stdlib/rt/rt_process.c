// Subprocess bridge: run a worker program, write JSON/text to stdin, read stdout.
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if !defined(_WIN32)
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>
#else
#include <windows.h>
#include <process.h>
#endif

static char *dup_bytes(const char *s, size_t n) {
    char *p = (char *)malloc(n + 1);
    if (!p) {
        return NULL;
    }
    memcpy(p, s, n);
    p[n] = '\0';
    return p;
}

#if !defined(_WIN32)
static char *bridge_run_child(
    const char *program,
    const char *arg1,
    const char *input) {
    if (!program || !*program || !input) {
        return NULL;
    }

    int inpipe[2];
    int outpipe[2];
    if (pipe(inpipe) != 0 || pipe(outpipe) != 0) {
        return NULL;
    }

    pid_t pid = fork();
    if (pid < 0) {
        close(inpipe[0]);
        close(inpipe[1]);
        close(outpipe[0]);
        close(outpipe[1]);
        return NULL;
    }

    if (pid == 0) {
        close(inpipe[1]);
        close(outpipe[0]);
        if (dup2(inpipe[0], STDIN_FILENO) < 0) {
            _exit(126);
        }
        if (dup2(outpipe[1], STDOUT_FILENO) < 0) {
            _exit(126);
        }
        close(inpipe[0]);
        close(outpipe[1]);
        if (arg1 && *arg1) {
            execlp(program, program, arg1, (char *)NULL);
        } else {
            execlp(program, program, (char *)NULL);
        }
        _exit(127);
    }

    // Write before child may exec so the pipe buffer is never empty at readline().
    size_t inlen = strlen(input);
    if (write(inpipe[1], input, inlen) < 0 || write(inpipe[1], "\n", 1) < 0) {
        close(inpipe[0]);
        close(inpipe[1]);
        close(outpipe[0]);
        close(outpipe[1]);
        waitpid(pid, NULL, 0);
        return NULL;
    }
    close(inpipe[0]);
    close(outpipe[1]);
    close(inpipe[1]);

    char buf[65536];
    size_t total = 0;
    while (total < sizeof(buf) - 1) {
        ssize_t n = read(outpipe[0], buf + total, sizeof(buf) - 1 - total);
        if (n <= 0) {
            break;
        }
        total += (size_t)n;
    }
    close(outpipe[0]);

    int status = 0;
    if (waitpid(pid, &status, 0) < 0) {
        return NULL;
    }
    if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) {
        return NULL;
    }

    while (total > 0 && (buf[total - 1] == '\n' || buf[total - 1] == '\r')) {
        total--;
    }
    return dup_bytes(buf, total);
}
#endif

char *rt_bridge_exec(const char *program, const char *input) {
#if defined(_WIN32)
    (void)program;
    (void)input;
    return NULL;
#else
    return bridge_run_child(program, NULL, input);
#endif
}

char *rt_bridge_exec_arg(const char *program, const char *arg1, const char *input) {
#if defined(_WIN32)
    (void)program;
    (void)arg1;
    (void)input;
    return NULL;
#else
    return bridge_run_child(program, arg1, input);
#endif
}

extern void *vec_str_new(void);
extern int vec_str_len(void *handle);
extern const char *vec_str_get(void *handle, int index);

int command_run(const char *program, void *args_handle) {
#if defined(_WIN32)
    if (!program || !*program) {
        return -1;
    }

    int extra = 0;
    if (args_handle) {
        extra = vec_str_len(args_handle);
        if (extra < 0) {
            extra = 0;
        }
    }
    if (extra > 30) {
        return -1;
    }

    size_t cap = strlen(program) + 4;
    for (int i = 0; i < extra; i++) {
        const char *arg = vec_str_get(args_handle, i);
        cap += (arg ? strlen(arg) : 0) + 3;
    }
    char *cmdline = (char *)malloc(cap);
    if (!cmdline) {
        return -1;
    }
    size_t pos = 0;
    pos += (size_t)snprintf(cmdline + pos, cap - pos, "\"%s\"", program);
    for (int i = 0; i < extra; i++) {
        const char *arg = vec_str_get(args_handle, i);
        if (!arg) {
            arg = "";
        }
        pos += (size_t)snprintf(cmdline + pos, cap - pos, " \"%s\"", arg);
    }

    STARTUPINFOA si;
    PROCESS_INFORMATION pi;
    ZeroMemory(&si, sizeof(si));
    si.cb = sizeof(si);
    ZeroMemory(&pi, sizeof(pi));

    if (!CreateProcessA(NULL, cmdline, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi)) {
        free(cmdline);
        return -1;
    }
    free(cmdline);

    WaitForSingleObject(pi.hProcess, INFINITE);
    DWORD code = 1;
    GetExitCodeProcess(pi.hProcess, &code);
    CloseHandle(pi.hProcess);
    CloseHandle(pi.hThread);
    return (int)code;
#else
    if (!program || !*program) {
        return -1;
    }

    int extra = 0;
    if (args_handle) {
        extra = vec_str_len(args_handle);
        if (extra < 0) {
            extra = 0;
        }
    }
    if (extra > 30) {
        return -1;
    }

    const char *argv[33];
    int argc = 0;
    argv[argc++] = program;
    for (int i = 0; i < extra; i++) {
        const char *arg = vec_str_get(args_handle, i);
        argv[argc++] = arg ? arg : "";
    }
    argv[argc] = NULL;

    pid_t pid = fork();
    if (pid < 0) {
        return -1;
    }
    if (pid == 0) {
        execvp(program, (char *const *)argv);
        _exit(127);
    }

    int status = 0;
    if (waitpid(pid, &status, 0) < 0) {
        return -1;
    }
    if (!WIFEXITED(status)) {
        return -1;
    }
    return WEXITSTATUS(status);
#endif
}

static char *json_escape_dup(const char *s) {
    if (!s) {
        return dup_bytes("", 0);
    }
    size_t len = strlen(s);
    size_t cap = len * 2 + 8;
    char *out = (char *)malloc(cap);
    if (!out) {
        return NULL;
    }
    size_t j = 0;
    for (size_t i = 0; i < len; i++) {
        unsigned char c = (unsigned char)s[i];
        char pair[3] = {0, 0, 0};
        if (c == '"' || c == '\\') {
            pair[0] = '\\';
            pair[1] = (char)c;
        } else if (c == '\n') {
            pair[0] = '\\';
            pair[1] = 'n';
        } else if (c == '\r') {
            pair[0] = '\\';
            pair[1] = 'r';
        } else if (c == '\t') {
            pair[0] = '\\';
            pair[1] = 't';
        } else {
            if (j + 1 >= cap) {
                cap *= 2;
                char *n = (char *)realloc(out, cap);
                if (!n) {
                    free(out);
                    return NULL;
                }
                out = n;
            }
            out[j++] = (char)c;
            continue;
        }
        if (j + 2 >= cap) {
            cap *= 2;
            char *n = (char *)realloc(out, cap);
            if (!n) {
                free(out);
                return NULL;
            }
            out = n;
        }
        out[j++] = pair[0];
        out[j++] = pair[1];
    }
    if (j + 1 >= cap) {
        cap += 1;
        char *n = (char *)realloc(out, cap);
        if (!n) {
            free(out);
            return NULL;
        }
        out = n;
    }
    out[j] = '\0';
    return out;
}

#if !defined(_WIN32)
static char *read_pipe_all(int fd, size_t max_cap) {
    char *buf = (char *)malloc(max_cap + 1);
    if (!buf) {
        return NULL;
    }
    size_t total = 0;
    while (total < max_cap) {
        ssize_t n = read(fd, buf + total, max_cap - total);
        if (n <= 0) {
            break;
        }
        total += (size_t)n;
    }
    buf[total] = '\0';
    return buf;
}

static char *command_exec_capture_posix(const char *program, void *args_handle) {
    if (!program || !*program) {
        return dup_bytes("{\"code\":-1,\"stdout\":\"\",\"stderr\":\"missing program\"}", 52);
    }

    int extra = 0;
    if (args_handle) {
        extra = vec_str_len(args_handle);
        if (extra < 0) {
            extra = 0;
        }
    }
    if (extra > 30) {
        return dup_bytes("{\"code\":-1,\"stdout\":\"\",\"stderr\":\"too many args\"}", 49);
    }

    const char *argv[33];
    int argc = 0;
    argv[argc++] = program;
    for (int i = 0; i < extra; i++) {
        const char *arg = vec_str_get(args_handle, i);
        argv[argc++] = arg ? arg : "";
    }
    argv[argc] = NULL;

    int outpipe[2];
    int errpipe[2];
    if (pipe(outpipe) != 0 || pipe(errpipe) != 0) {
        return dup_bytes("{\"code\":-1,\"stdout\":\"\",\"stderr\":\"pipe failed\"}", 47);
    }

    pid_t pid = fork();
    if (pid < 0) {
        close(outpipe[0]);
        close(outpipe[1]);
        close(errpipe[0]);
        close(errpipe[1]);
        return dup_bytes("{\"code\":-1,\"stdout\":\"\",\"stderr\":\"fork failed\"}", 48);
    }

    if (pid == 0) {
        close(outpipe[0]);
        close(errpipe[0]);
        if (dup2(outpipe[1], STDOUT_FILENO) < 0) {
            _exit(126);
        }
        if (dup2(errpipe[1], STDERR_FILENO) < 0) {
            _exit(126);
        }
        close(outpipe[1]);
        close(errpipe[1]);
        execvp(program, (char *const *)argv);
        _exit(127);
    }

    close(outpipe[1]);
    close(errpipe[1]);
    char *stdout_buf = read_pipe_all(outpipe[0], 1024 * 1024);
    char *stderr_buf = read_pipe_all(errpipe[0], 1024 * 1024);
    close(outpipe[0]);
    close(errpipe[0]);

    int status = 0;
    if (waitpid(pid, &status, 0) < 0) {
        free(stdout_buf);
        free(stderr_buf);
        return dup_bytes("{\"code\":-1,\"stdout\":\"\",\"stderr\":\"wait failed\"}", 47);
    }

    int code = -1;
    if (WIFEXITED(status)) {
        code = WEXITSTATUS(status);
    }

    char *stdout_esc = json_escape_dup(stdout_buf ? stdout_buf : "");
    char *stderr_esc = json_escape_dup(stderr_buf ? stderr_buf : "");
    free(stdout_buf);
    free(stderr_buf);
    if (!stdout_esc || !stderr_esc) {
        free(stdout_esc);
        free(stderr_esc);
        return dup_bytes("{\"code\":-1,\"stdout\":\"\",\"stderr\":\"oom\"}", 41);
    }

    size_t cap = strlen(stdout_esc) + strlen(stderr_esc) + 64;
    char *json = (char *)malloc(cap);
    if (!json) {
        free(stdout_esc);
        free(stderr_esc);
        return NULL;
    }
    snprintf(json, cap, "{\"code\":%d,\"stdout\":\"%s\",\"stderr\":\"%s\"}", code, stdout_esc,
             stderr_esc);
    free(stdout_esc);
    free(stderr_esc);
    return json;
}
#endif

char *command_exec_capture(const char *program, void *args_handle) {
#if defined(_WIN32)
    (void)program;
    (void)args_handle;
    return dup_bytes("{\"code\":-1,\"stdout\":\"\",\"stderr\":\"not supported on Windows\"}", 58);
#else
    return command_exec_capture_posix(program, args_handle);
#endif
}
