// Fetch options: custom headers, timeout, redirect, abort, cookies.
import "headers.ny"
import "cookies.ny"
import "abort.ny"
import "form.ny"
import "types.ny"

const REDIRECT_FOLLOW = 1
const REDIRECT_ERROR = 2
const REDIRECT_MANUAL = 3

struct RequestInit {
    method: i32
    headers: HashMap_str_str
    body: string
    content_type: string
    timeout_ms: i32
    redirect: i32
    max_redirects: i32
    signal: AbortSignal
    jar: CookieJar
}

fn RequestInit_new() -> RequestInit {
    return RequestInit {
        method: METHOD_GET,
        headers: HeaderMap_new(),
        body: "",
        content_type: "",
        timeout_ms: 30000,
        redirect: REDIRECT_FOLLOW,
        max_redirects: 5,
        signal: AbortSignal_new(),
        jar: CookieJar_new(),
    }
}

fn RequestInit_method(init: RequestInit, method: i32) -> RequestInit {
    return RequestInit {
        method: method,
        headers: init.headers,
        body: init.body,
        content_type: init.content_type,
        timeout_ms: init.timeout_ms,
        redirect: init.redirect,
        max_redirects: init.max_redirects,
        signal: init.signal,
        jar: init.jar,
    }
}

fn RequestInit_header(init: RequestInit, name: string, value: string) -> RequestInit {
    return RequestInit {
        method: init.method,
        headers: HeaderMap_set(init.headers, name, value),
        body: init.body,
        content_type: init.content_type,
        timeout_ms: init.timeout_ms,
        redirect: init.redirect,
        max_redirects: init.max_redirects,
        signal: init.signal,
        jar: init.jar,
    }
}

fn RequestInit_authorization(init: RequestInit, token: string) -> RequestInit {
    return RequestInit_header(init, "Authorization", token)
}

fn RequestInit_cookie(init: RequestInit, cookie: string) -> RequestInit {
    return RequestInit_header(init, "Cookie", cookie)
}

fn RequestInit_body(init: RequestInit, body: string, content_type: string) -> RequestInit {
    return RequestInit {
        method: init.method,
        headers: init.headers,
        body: body,
        content_type: content_type,
        timeout_ms: init.timeout_ms,
        redirect: init.redirect,
        max_redirects: init.max_redirects,
        signal: init.signal,
        jar: init.jar,
    }
}

fn RequestInit_json(init: RequestInit, body: string) -> RequestInit {
    return RequestInit_body(init, body, "application/json")
}

fn RequestInit_form(init: RequestInit, form: FormData) -> RequestInit {
    return RequestInit_body(init, FormData_to_urlencoded(form), "application/x-www-form-urlencoded")
}

fn RequestInit_multipart(init: RequestInit, form: FormData, boundary: string) -> RequestInit {
    return RequestInit_body(
        init,
        FormData_to_multipart(form, boundary),
        FormData_content_type_multipart(boundary)
    )
}

fn RequestInit_timeout(init: RequestInit, timeout_ms: i32) -> RequestInit {
    return RequestInit {
        method: init.method,
        headers: init.headers,
        body: init.body,
        content_type: init.content_type,
        timeout_ms: timeout_ms,
        redirect: init.redirect,
        max_redirects: init.max_redirects,
        signal: init.signal,
        jar: init.jar,
    }
}

fn RequestInit_redirect(init: RequestInit, mode: i32) -> RequestInit {
    return RequestInit {
        method: init.method,
        headers: init.headers,
        body: init.body,
        content_type: init.content_type,
        timeout_ms: init.timeout_ms,
        redirect: mode,
        max_redirects: init.max_redirects,
        signal: init.signal,
        jar: init.jar,
    }
}

fn RequestInit_signal(init: RequestInit, signal: AbortSignal) -> RequestInit {
    return RequestInit {
        method: init.method,
        headers: init.headers,
        body: init.body,
        content_type: init.content_type,
        timeout_ms: init.timeout_ms,
        redirect: init.redirect,
        max_redirects: init.max_redirects,
        signal: signal,
        jar: init.jar,
    }
}

fn RequestInit_cookie_jar(init: RequestInit, jar: CookieJar) -> RequestInit {
    return RequestInit {
        method: init.method,
        headers: init.headers,
        body: init.body,
        content_type: init.content_type,
        timeout_ms: init.timeout_ms,
        redirect: init.redirect,
        max_redirects: init.max_redirects,
        signal: init.signal,
        jar: jar,
    }
}
