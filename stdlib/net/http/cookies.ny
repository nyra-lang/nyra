// Cookie jar — store Set-Cookie / Cookie for HTTP clients.
import "../../map.ny"
import "../../strings.ny"
import "headers.ny"

struct CookieJar {
    cookies: HashMap_str_str
}

fn CookieJar_new() -> CookieJar {
    return CookieJar { cookies: HashMap_str_str_new() }
}

fn CookieJar_set(jar: CookieJar, name: string, value: string) -> CookieJar {
    return CookieJar { cookies: jar.cookies.insert(name, value) }
}

fn CookieJar_get(jar: CookieJar, name: string) -> string {
    if jar.cookies.contains(name) == 1 {
        return jar.cookies.get(name)
    }
    return ""
}

fn CookieJar_header(jar: CookieJar) -> string {
    let keys = jar.cookies.keys()
    let n = keys.len()
    let mut out = ""
    let mut i = 0
    while i < n {
        let k = keys.get(i)
        let v = jar.cookies.get(k)
        if i > 0 {
            out = strcat(out, "; ")
        }
        out = strcat(out, strcat(strcat(k, "="), v))
        i = i + 1
    }
    return out
}

fn CookieJar_apply_set_cookie(jar: CookieJar, set_cookie: string) -> CookieJar {
    let mut rest = set_cookie
    let semi = strstr_pos(rest, ";")
    if semi >= 0 {
        rest = substring(rest, 0, semi)
    }
    let eq = strstr_pos(rest, "=")
    if eq < 0 {
        return jar
    }
    let name = substring(rest, 0, eq)
    let value = substring(rest, eq + 1, strlen(rest) - (eq + 1))
    return CookieJar_set(jar, name, value)
}

fn CookieJar_absorb_response(jar: CookieJar, headers: HashMap_str_str) -> CookieJar {
    let sc = HeaderMap_get(headers, "Set-Cookie")
    if strlen(sc) == 0 {
        return jar
    }
    return CookieJar_apply_set_cookie(jar, sc)
}
