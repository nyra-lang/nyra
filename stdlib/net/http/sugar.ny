// Ergonomic HTTP — one entry (`fetch` / `req`) like JS; long names are optional aliases.
import "headers.ny"
import "form.ny"
import "cookies.ny"
import "abort.ny"
import "body.ny"
import "fetch.ny"
import "types.ny"
import "response.ny"
import "client.ny"

// --- short constructors ---

fn req() -> RequestInit {
    return RequestInit_new()
}

fn form() -> FormData {
    return FormData_new()
}

fn params() -> URLSearchParams {
    return URLSearchParams_new()
}

fn cookies() -> CookieJar {
    return CookieJar_new()
}

fn headers() -> HashMap_str_str {
    return HeaderMap_new()
}

// --- fluent RequestInit (configure then fire with a verb that takes the URL) ---

impl RequestInit {
    // Set method from string: "GET", "POST", "PATCH", …
    fn verb(self, name: string) -> RequestInit {
        return RequestInit_method(self, method_from_name(name))
    }

    fn method(self, method: i32) -> RequestInit {
        return RequestInit_method(self, method)
    }

    fn get(self, url: string) -> HttpResponse {
        return fetch_with(url, RequestInit_method(self, METHOD_GET))
    }

    fn post(self, url: string) -> HttpResponse {
        return fetch_with(url, RequestInit_method(self, METHOD_POST))
    }

    fn put(self, url: string) -> HttpResponse {
        return fetch_with(url, RequestInit_method(self, METHOD_PUT))
    }

    fn patch(self, url: string) -> HttpResponse {
        return fetch_with(url, RequestInit_method(self, METHOD_PATCH))
    }

    fn delete(self, url: string) -> HttpResponse {
        return fetch_with(url, RequestInit_method(self, METHOD_DELETE))
    }

    fn head(self, url: string) -> HttpResponse {
        return fetch_with(url, RequestInit_method(self, METHOD_HEAD))
    }

    // Fire with whatever method is already set on the init (default GET).
    fn go(self, url: string) -> HttpResponse {
        return fetch_with(url, self)
    }

    // Alias of go (kept for older snippets).
    fn send(self, url: string) -> HttpResponse {
        return fetch_with(url, self)
    }

    fn header(self, name: string, value: string) -> RequestInit {
        return RequestInit_header(self, name, value)
    }

    fn authorization(self, token: string) -> RequestInit {
        return RequestInit_authorization(self, token)
    }

    fn cookie(self, cookie: string) -> RequestInit {
        return RequestInit_cookie(self, cookie)
    }

    fn body(self, body: string, content_type: string) -> RequestInit {
        return RequestInit_body(self, body, content_type)
    }

    fn json(self, body: string) -> RequestInit {
        return RequestInit_json(self, body)
    }

    fn form(self, form: FormData) -> RequestInit {
        return RequestInit_form(self, form)
    }

    fn multipart(self, form: FormData, boundary: string) -> RequestInit {
        return RequestInit_multipart(self, form, boundary)
    }

    fn timeout(self, timeout_ms: i32) -> RequestInit {
        return RequestInit_timeout(self, timeout_ms)
    }

    fn redirect(self, mode: i32) -> RequestInit {
        return RequestInit_redirect(self, mode)
    }

    fn signal(self, signal: AbortSignal) -> RequestInit {
        return RequestInit_signal(self, signal)
    }

    fn cookie_jar(self, jar: CookieJar) -> RequestInit {
        return RequestInit_cookie_jar(self, jar)
    }
}

// --- fluent FormData / URLSearchParams / CookieJar / abort ---

impl FormData {
    fn append(self, name: string, value: string) -> FormData {
        return FormData_append(self, name, value)
    }

    fn set(self, name: string, value: string) -> FormData {
        return FormData_set(self, name, value)
    }

    fn get(self, name: string) -> string {
        return FormData_get(self, name)
    }

    fn urlencoded(self) -> string {
        return FormData_to_urlencoded(self)
    }
}

impl URLSearchParams {
    fn append(self, name: string, value: string) -> URLSearchParams {
        return URLSearchParams_append(self, name, value)
    }

    fn set(self, name: string, value: string) -> URLSearchParams {
        return URLSearchParams_set(self, name, value)
    }

    fn get(self, name: string) -> string {
        return URLSearchParams_get(self, name)
    }

    fn to_string(self) -> string {
        return URLSearchParams_to_string(self)
    }
}

impl CookieJar {
    fn set(self, name: string, value: string) -> CookieJar {
        return CookieJar_set(self, name, value)
    }

    fn get(self, name: string) -> string {
        return CookieJar_get(self, name)
    }

    fn header(self) -> string {
        return CookieJar_header(self)
    }
}

impl AbortController {
    fn abort(self) -> AbortController {
        return AbortController_abort(self)
    }

    fn signal(self) -> AbortSignal {
        return AbortController_signal(self)
    }
}

// --- HttpResponse ---

impl HttpResponse {
    fn text(self) -> string {
        return HttpResponse_text(self)
    }

    fn json(self) -> HashMap_str_str {
        return HttpResponse_json(self)
    }

    fn header(self, name: string) -> string {
        return HttpResponse_header(self, name)
    }

    fn blob(self) -> Blob {
        return HttpResponse_blob(self)
    }

    fn is_ok(self) -> i32 {
        if self.status >= 200 && self.status < 300 {
            return 1
        }
        return 0
    }
}

// --- short free helpers (no http_ prefix) ---

fn get_json(url: string) -> HashMap_str_str {
    return HttpResponse_json(fetch(url))
}

fn post_json(url: string, body: string) -> HttpResponse {
    return req().json(body).post(url)
}

fn post_form(url: string, form: FormData) -> HttpResponse {
    return req().form(form).post(url)
}

fn put_json(url: string, body: string) -> HttpResponse {
    return req().json(body).put(url)
}

fn patch_json(url: string, body: string) -> HttpResponse {
    return req().json(body).patch(url)
}

// --- legacy long names (compat; prefer fetch / req / post_json) ---

fn http_get(url: string) -> HttpResponse {
    return fetch(url)
}

fn http_get_auth(url: string, token: string) -> HttpResponse {
    return req().authorization(token).get(url)
}

fn http_post_json(url: string, body: string) -> HttpResponse {
    return post_json(url, body)
}

fn http_post_form(url: string, form: FormData) -> HttpResponse {
    return post_form(url, form)
}

fn http_put_json(url: string, body: string) -> HttpResponse {
    return put_json(url, body)
}

fn http_patch_json(url: string, body: string) -> HttpResponse {
    return patch_json(url, body)
}

fn http_delete(url: string) -> HttpResponse {
    return req().delete(url)
}

fn http_get_json(url: string) -> HashMap_str_str {
    return get_json(url)
}

// jstr / jraw / jobj / jnum / jparse live in stdlib/json/mod.ny (shared).
