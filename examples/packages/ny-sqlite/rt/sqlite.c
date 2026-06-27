#include <sqlite3.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* NyraPkg shim — compiled via `link-source rt/sqlite.c` in nyra.mod */

static sqlite3 *db_from_handle(int handle) {
    return (sqlite3 *)(intptr_t)handle;
}

int sqlite_open(const char *path) {
    sqlite3 *db = NULL;
    if (sqlite3_open(path, &db) != SQLITE_OK) {
        if (db) {
            sqlite3_close(db);
        }
        return 0;
    }
    return (int)(intptr_t)db;
}

int sqlite_exec(int handle, const char *sql) {
    sqlite3 *db = db_from_handle(handle);
    if (!db || !sql) {
        return -1;
    }
    char *err = NULL;
    int rc = sqlite3_exec(db, sql, NULL, NULL, &err);
    if (err) {
        sqlite3_free(err);
    }
    return rc == SQLITE_OK ? 0 : rc;
}

void sqlite_close(int handle) {
    sqlite3 *db = db_from_handle(handle);
    if (db) {
        sqlite3_close(db);
    }
}
