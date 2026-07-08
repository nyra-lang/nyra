// FormData, URLSearchParams, and application/x-www-form-urlencoded helpers.
import "../../map.ny"
import "../../strings.ny"
import "../../vec_str.ny"
import "../../encoding/mod.ny"
import "../../mime/mod.ny"

struct FormData {
    fields: HashMap_str_str
    files: HashMap_str_str
}

struct URLSearchParams {
    params: HashMap_str_str
}

fn FormData_new() -> FormData {
    return FormData { fields: HashMap_str_str_new(), files: HashMap_str_str_new() }
}

fn FormData_append(form: FormData, name: string, value: string) -> FormData {
    return FormData { fields: form.fields.insert(name, value), files: form.files }
}

fn FormData_set(form: FormData, name: string, value: string) -> FormData {
    return FormData_append(form, name, value)
}

fn FormData_get(form: FormData, name: string) -> string {
    if form.fields.contains(name) == 1 {
        return form.fields.get(name)
    }
    return ""
}

fn FormData_has(form: FormData, name: string) -> i32 {
    return form.fields.contains(name)
}

fn FormData_append_file(form: FormData, name: string, filename: string, content: string) -> FormData {
    let payload = strcat(strcat(filename, "\n"), content)
    return FormData { fields: form.fields, files: form.files.insert(name, payload) }
}

fn FormData_to_urlencoded(form: FormData) -> string {
    return URLSearchParams_to_string(URLSearchParams { params: form.fields })
}

fn FormData_to_multipart(form: FormData, boundary: string) -> string {
    let keys = form.fields.keys()
    let n = keys.len()
    let mut pairs = StrVec_new()
    let mut i = 0
    while i < n {
        let k = keys.get(i)
        let v = form.fields.get(k)
        pairs = pairs.push(strcat(strcat(k, "="), v))
        i = i + 1
    }
    let mut out = mime_write_multipart(boundary, pairs)
    let fkeys = form.files.keys()
    let fn_ = fkeys.len()
    let mut j = 0
    while j < fn_ {
        let name = fkeys.get(j)
        let payload = form.files.get(name)
        let nl = strstr_pos(payload, "\n")
        let mut filename = "blob"
        let mut content = payload
        if nl >= 0 {
            filename = substring(payload, 0, nl)
            content = substring(payload, nl + 1, strlen(payload) - (nl + 1))
        }
        let part = strcat(
            strcat(
                strcat(strcat("--", boundary), "\r\nContent-Disposition: form-data; name=\""),
                strcat(strcat(name, "\"; filename=\""), strcat(filename, "\"\r\nContent-Type: application/octet-stream\r\n\r\n"))
            ),
            strcat(content, "\r\n")
        )
        // Rebuild: drop closing boundary then append file parts + close.
        let close = strcat(strcat("--", boundary), "--\r\n")
        let cut = strstr_pos(out, close)
        if cut >= 0 {
            out = substring(out, 0, cut)
        }
        out = strcat(strcat(out, part), close)
        j = j + 1
    }
    return out
}

fn FormData_content_type_multipart(boundary: string) -> string {
    return mime_content_type_multipart(boundary)
}

fn URLSearchParams_new() -> URLSearchParams {
    return URLSearchParams { params: HashMap_str_str_new() }
}

fn URLSearchParams_from_string(qs: string) -> URLSearchParams {
    let mut params = HashMap_str_str_new()
    let mut rest = qs
    if strlen(rest) > 0 && char_at(rest, 0) == 63 {
        rest = substring(rest, 1, strlen(rest) - 1)
    }
    while strlen(rest) > 0 {
        let amp = strstr_pos(rest, "&")
        let mut pair = rest
        if amp >= 0 {
            pair = substring(rest, 0, amp)
            rest = substring(rest, amp + 1, strlen(rest) - (amp + 1))
        } else {
            rest = ""
        }
        let eq = strstr_pos(pair, "=")
        if eq >= 0 {
            let k = url_decode(substring(pair, 0, eq))
            let v = url_decode(substring(pair, eq + 1, strlen(pair) - (eq + 1)))
            params = params.insert(k, v)
        } else {
            if strlen(pair) > 0 {
                params = params.insert(url_decode(pair), "")
            }
        }
    }
    return URLSearchParams { params: params }
}

fn URLSearchParams_append(p: URLSearchParams, name: string, value: string) -> URLSearchParams {
    return URLSearchParams { params: p.params.insert(name, value) }
}

fn URLSearchParams_set(p: URLSearchParams, name: string, value: string) -> URLSearchParams {
    return URLSearchParams_append(p, name, value)
}

fn URLSearchParams_get(p: URLSearchParams, name: string) -> string {
    if p.params.contains(name) == 1 {
        return p.params.get(name)
    }
    return ""
}

fn URLSearchParams_has(p: URLSearchParams, name: string) -> i32 {
    return p.params.contains(name)
}

fn URLSearchParams_to_string(p: URLSearchParams) -> string {
    let keys = p.params.keys()
    let n = keys.len()
    let mut out = ""
    let mut i = 0
    while i < n {
        let k = keys.get(i)
        let v = p.params.get(k)
        if i > 0 {
            out = strcat(out, "&")
        }
        out = strcat(out, strcat(strcat(url_encode(k), "="), url_encode(v)))
        i = i + 1
    }
    return out
}

fn form_urlencoded_encode(map: HashMap_str_str) -> string {
    return URLSearchParams_to_string(URLSearchParams { params: map })
}

fn form_urlencoded_decode(body: string) -> HashMap_str_str {
    return URLSearchParams_from_string(body).params
}
