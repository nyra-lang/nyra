// Windows ConPTY implementation (included from rt_pty.c when _WIN32).
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <io.h>
#include <fcntl.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

#define PTY_MAX_TRACKED 8

typedef void *HPCON;
typedef HRESULT(WINAPI *PFN_CreatePseudoConsole)(COORD, HANDLE, HANDLE, DWORD, HPCON *);
typedef HRESULT(WINAPI *PFN_ResizePseudoConsole)(HPCON, COORD);
typedef void(WINAPI *PFN_ClosePseudoConsole)(HPCON);

#ifndef PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE
#define PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE 0x00020016
#endif

struct win_pty_entry {
    int master_fd;
    int active;
    HPCON hPC;
    HANDLE hInputWrite;
    HANDLE hOutputRead;
    HANDLE hProcess;
};

static struct win_pty_entry g_win_pty[PTY_MAX_TRACKED];
static PFN_CreatePseudoConsole pCreatePseudoConsole = NULL;
static PFN_ResizePseudoConsole pResizePseudoConsole = NULL;
static PFN_ClosePseudoConsole pClosePseudoConsole = NULL;
static int g_conpty_loaded = 0;

static int conpty_load(void) {
    if (g_conpty_loaded) {
        return pCreatePseudoConsole ? 1 : 0;
    }
    g_conpty_loaded = 1;
    HMODULE k32 = GetModuleHandleA("kernel32.dll");
    if (!k32) {
        return 0;
    }
    pCreatePseudoConsole =
        (PFN_CreatePseudoConsole)(void *)GetProcAddress(k32, "CreatePseudoConsole");
    pResizePseudoConsole =
        (PFN_ResizePseudoConsole)(void *)GetProcAddress(k32, "ResizePseudoConsole");
    pClosePseudoConsole =
        (PFN_ClosePseudoConsole)(void *)GetProcAddress(k32, "ClosePseudoConsole");
    return pCreatePseudoConsole && pClosePseudoConsole ? 1 : 0;
}

static struct win_pty_entry *win_pty_find(int master) {
    for (int i = 0; i < PTY_MAX_TRACKED; i++) {
        if (g_win_pty[i].active && g_win_pty[i].master_fd == master) {
            return &g_win_pty[i];
        }
    }
    return NULL;
}

static struct win_pty_entry *win_pty_alloc(void) {
    for (int i = 0; i < PTY_MAX_TRACKED; i++) {
        if (!g_win_pty[i].active) {
            return &g_win_pty[i];
        }
    }
    return NULL;
}

static char *pty_empty_string(void) {
    char *empty = (char *)malloc(1);
    if (empty) {
        empty[0] = '\0';
    }
    return empty;
}

int pty_spawn(const char *shell, int rows, int cols) {
    if (!conpty_load()) {
        return -1;
    }
    SECURITY_ATTRIBUTES sa = {sizeof(sa), NULL, TRUE};
    HANDLE hOutputRead = NULL;
    HANDLE hOutputWrite = NULL;
    HANDLE hInputRead = NULL;
    HANDLE hInputWrite = NULL;
    if (!CreatePipe(&hOutputRead, &hOutputWrite, &sa, 0)) {
        return -1;
    }
    if (!CreatePipe(&hInputRead, &hInputWrite, &sa, 0)) {
        CloseHandle(hOutputRead);
        CloseHandle(hOutputWrite);
        return -1;
    }
    COORD size = {(SHORT)(cols > 0 ? cols : 80), (SHORT)(rows > 0 ? rows : 24)};
    HPCON hPC = NULL;
    HRESULT hr = pCreatePseudoConsole(size, hInputWrite, hOutputRead, 0, &hPC);
    if (FAILED(hr) || !hPC) {
        CloseHandle(hOutputRead);
        CloseHandle(hOutputWrite);
        CloseHandle(hInputRead);
        CloseHandle(hInputWrite);
        return -1;
    }
    SIZE_T attrSize = 0;
    InitializeProcThreadAttributeList(NULL, 1, 0, &attrSize);
    LPPROC_THREAD_ATTRIBUTE_LIST attrList =
        (LPPROC_THREAD_ATTRIBUTE_LIST)HeapAlloc(GetProcessHeap(), 0, attrSize);
    if (!attrList ||
        !InitializeProcThreadAttributeList(attrList, 1, 0, &attrSize)) {
        pClosePseudoConsole(hPC);
        CloseHandle(hOutputRead);
        CloseHandle(hOutputWrite);
        CloseHandle(hInputRead);
        CloseHandle(hInputWrite);
        return -1;
    }
    if (!UpdateProcThreadAttribute(attrList, 0, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, hPC,
                                   sizeof(HPCON), NULL, NULL)) {
        DeleteProcThreadAttributeList(attrList);
        HeapFree(GetProcessHeap(), 0, attrList);
        pClosePseudoConsole(hPC);
        CloseHandle(hOutputRead);
        CloseHandle(hOutputWrite);
        CloseHandle(hInputRead);
        CloseHandle(hInputWrite);
        return -1;
    }
    STARTUPINFOEXA si = {0};
    si.StartupInfo.cb = sizeof(si);
    si.lpAttributeList = attrList;
    PROCESS_INFORMATION pi = {0};
    const char *cmd = (shell && shell[0]) ? shell : "cmd.exe";
    char cmdline[512];
    snprintf(cmdline, sizeof(cmdline), "%s", cmd);
    BOOL ok = CreateProcessA(NULL, cmdline, NULL, NULL, FALSE,
                             EXTENDED_STARTUPINFO_PRESENT, NULL, NULL,
                             &si.StartupInfo, &pi);
    DeleteProcThreadAttributeList(attrList);
    HeapFree(GetProcessHeap(), 0, attrList);
    CloseHandle(hInputRead);
    CloseHandle(hOutputWrite);
    if (!ok) {
        pClosePseudoConsole(hPC);
        CloseHandle(hOutputRead);
        CloseHandle(hInputWrite);
        return -1;
    }
    CloseHandle(pi.hThread);
    int master = _open_osfhandle((intptr_t)hOutputRead, O_RDONLY | O_TEXT);
    if (master < 0) {
        TerminateProcess(pi.hProcess, 1);
        CloseHandle(pi.hProcess);
        pClosePseudoConsole(hPC);
        CloseHandle(hInputWrite);
        return -1;
    }
    struct win_pty_entry *slot = win_pty_alloc();
    if (!slot) {
        _close(master);
        TerminateProcess(pi.hProcess, 1);
        CloseHandle(pi.hProcess);
        pClosePseudoConsole(hPC);
        CloseHandle(hInputWrite);
        return -1;
    }
    slot->master_fd = master;
    slot->active = 1;
    slot->hPC = hPC;
    slot->hInputWrite = hInputWrite;
    slot->hOutputRead = hOutputRead;
    slot->hProcess = pi.hProcess;
    return master;
}

int pty_write(int master, const char *data) {
    struct win_pty_entry *e = win_pty_find(master);
    if (!e || !data) {
        return -1;
    }
    DWORD written = 0;
    DWORD len = (DWORD)strlen(data);
    if (!WriteFile(e->hInputWrite, data, len, &written, NULL)) {
        return -1;
    }
    return (int)written;
}

static char *pty_read_handle(HANDLE h, int max_bytes) {
    if (!h || max_bytes <= 0) {
        return pty_empty_string();
    }
    char *buf = (char *)malloc((size_t)max_bytes + 1);
    if (!buf) {
        return pty_empty_string();
    }
    DWORD n = 0;
    if (!ReadFile(h, buf, (DWORD)max_bytes, &n, NULL) || n == 0) {
        buf[0] = '\0';
        return buf;
    }
    buf[n] = '\0';
    return buf;
}

char *pty_read(int master, int max_bytes) {
    struct win_pty_entry *e = win_pty_find(master);
    if (!e) {
        return pty_empty_string();
    }
    return pty_read_handle(e->hOutputRead, max_bytes > 0 ? max_bytes : 4096);
}

char *pty_drain(int master, int max_bytes) {
    return pty_read(master, max_bytes);
}

char *pty_drain_raw(int master, int max_bytes) {
    return pty_read(master, max_bytes);
}

int pty_poll(int master) {
    struct win_pty_entry *e = win_pty_find(master);
    if (!e) {
        return 0;
    }
    DWORD avail = 0;
    if (PeekNamedPipe(e->hOutputRead, NULL, 0, NULL, &avail, NULL) && avail > 0) {
        return 1;
    }
    return 0;
}

void pty_resize(int master, int rows, int cols) {
    struct win_pty_entry *e = win_pty_find(master);
    if (!e || !pResizePseudoConsole) {
        return;
    }
    COORD size = {(SHORT)(cols > 0 ? cols : 80), (SHORT)(rows > 0 ? rows : 24)};
    pResizePseudoConsole(e->hPC, size);
}

void pty_close(int master) {
    struct win_pty_entry *e = win_pty_find(master);
    if (!e) {
        return;
    }
    if (e->hInputWrite) {
        CloseHandle(e->hInputWrite);
    }
    if (e->hOutputRead) {
        CloseHandle(e->hOutputRead);
    }
    if (e->hPC && pClosePseudoConsole) {
        pClosePseudoConsole(e->hPC);
    }
    if (e->hProcess) {
        TerminateProcess(e->hProcess, 0);
        CloseHandle(e->hProcess);
    }
    if (master >= 0) {
        _close(master);
    }
    memset(e, 0, sizeof(*e));
}

int pty_wait(int master) {
    struct win_pty_entry *e = win_pty_find(master);
    if (!e || !e->hProcess) {
        return 0;
    }
    DWORD code = STILL_ACTIVE;
    if (WaitForSingleObject(e->hProcess, 0) == WAIT_OBJECT_0) {
        GetExitCodeProcess(e->hProcess, &code);
        return code == STILL_ACTIVE ? 0 : 1;
    }
    return 0;
}

char *pty_read_wait(int master, int max_bytes, int timeout_ms) {
    struct win_pty_entry *e = win_pty_find(master);
    if (!e) {
        return pty_empty_string();
    }
    DWORD start = GetTickCount();
    for (;;) {
        DWORD avail = 0;
        if (PeekNamedPipe(e->hOutputRead, NULL, 0, NULL, &avail, NULL) && avail > 0) {
            return pty_read_handle(e->hOutputRead, max_bytes > 0 ? max_bytes : 4096);
        }
        if (timeout_ms >= 0 && (int)(GetTickCount() - start) >= timeout_ms) {
            return pty_empty_string();
        }
        Sleep(10);
    }
}

char *pty_read_wait_raw(int master, int max_bytes, int timeout_ms) {
    return pty_read_wait(master, max_bytes, timeout_ms);
}

void pty_flush_stdout(int master, int max_bytes, int timeout_ms) {
    (void)max_bytes;
    (void)timeout_ms;
    struct win_pty_entry *e = win_pty_find(master);
    if (!e) {
        return;
    }
    DWORD start = GetTickCount();
    while (timeout_ms < 0 || (int)(GetTickCount() - start) < timeout_ms) {
        DWORD avail = 0;
        if (!PeekNamedPipe(e->hOutputRead, NULL, 0, NULL, &avail, NULL) || avail == 0) {
            break;
        }
        char scratch[256];
        DWORD n = 0;
        ReadFile(e->hOutputRead, scratch, sizeof(scratch), &n, NULL);
        if (n == 0) {
            break;
        }
    }
}
