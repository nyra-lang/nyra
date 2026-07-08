#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined(__has_include)
#if __has_include(<libpq-fe.h>)
#define NYRA_HAVE_POSTGRES 1
#include <libpq-fe.h>
#endif
#if __has_include(<mysql/mysql.h>)
#define NYRA_HAVE_MYSQL 1
#include <mysql/mysql.h>
#endif
#endif

void *_sqlite_null_handle(void) {
    return NULL;
}

void *_postgres_stub_open(const char *dsn) {
    if (!dsn || !*dsn) {
        return NULL;
    }
#ifdef NYRA_HAVE_POSTGRES
    PGconn *pg = PQconnectdb(dsn);
    if (PQstatus(pg) != CONNECTION_OK) {
        PQfinish(pg);
        return NULL;
    }
    return pg;
#else
    (void)dsn;
    return NULL;
#endif
}

int32_t postgres_exec(void *handle, const char *sql) {
    if (!handle || !sql) {
        return -1;
    }
#ifdef NYRA_HAVE_POSTGRES
    PGresult *res = PQexec((PGconn *)handle, sql);
    ExecStatusType st = PQresultStatus(res);
    PQclear(res);
    return (st == PGRES_COMMAND_OK || st == PGRES_TUPLES_OK) ? 0 : -1;
#else
    (void)handle;
    (void)sql;
    return -1;
#endif
}

void postgres_close(void *handle) {
    if (!handle) {
        return;
    }
#ifdef NYRA_HAVE_POSTGRES
    PQfinish((PGconn *)handle);
#else
    (void)handle;
#endif
}

void *_mysql_stub_open(const char *dsn) {
    if (!dsn || !*dsn) {
        return NULL;
    }
#ifdef NYRA_HAVE_MYSQL
    char host[128] = "127.0.0.1";
    char user[128] = "root";
    char pass[128] = "";
    char dbname[128] = "";
    unsigned int port = 3306;
    sscanf(dsn, "%127[^;];%u;%127[^;];%127[^;];%127[^;]", host, &port, dbname, user, pass);
    MYSQL *my = mysql_init(NULL);
    if (!my || !mysql_real_connect(my, host, user, pass, dbname, port, NULL, 0)) {
        if (my) {
            mysql_close(my);
        }
        return NULL;
    }
    return my;
#else
    (void)dsn;
    return NULL;
#endif
}

int32_t mysql_exec(void *handle, const char *sql) {
    if (!handle || !sql) {
        return -1;
    }
#ifdef NYRA_HAVE_MYSQL
    return mysql_query((MYSQL *)handle, sql) == 0 ? 0 : -1;
#else
    (void)handle;
    (void)sql;
    return -1;
#endif
}

/* Named `nyra_mysql_close` (not `mysql_close`) so the definition never clashes
 * with libmysqlclient's own `void mysql_close(MYSQL *)` from <mysql/mysql.h>
 * (which is in scope when NYRA_HAVE_MYSQL is set). Same rationale as nyra_atoi. */
void nyra_mysql_close(void *handle) {
    if (!handle) {
        return;
    }
#ifdef NYRA_HAVE_MYSQL
    mysql_close((MYSQL *)handle);
#else
    (void)handle;
#endif
}
