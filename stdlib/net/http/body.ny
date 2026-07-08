// Blob / ArrayBuffer / stream-like helpers for HTTP bodies (string-backed MVP).
// Full bytes-typed fields are deferred until LLVM struct emission supports `bytes`.

struct Blob {
    data: string
    content_type: string
}

struct ArrayBuffer {
    data: string
}

struct BodyStream {
    data: string
    offset: i32
}

fn Blob_from_string(s: string, content_type: string) -> Blob {
    return Blob { data: s, content_type: content_type }
}

fn Blob_text(blob: Blob) -> string {
    return blob.data
}

fn Blob_size(blob: Blob) -> i32 {
    return strlen(blob.data)
}

fn Blob_type(blob: Blob) -> string {
    return blob.content_type
}

fn ArrayBuffer_from_string(s: string) -> ArrayBuffer {
    return ArrayBuffer { data: s }
}

fn ArrayBuffer_byte_length(buf: ArrayBuffer) -> i32 {
    return strlen(buf.data)
}

fn ArrayBuffer_to_string(buf: ArrayBuffer) -> string {
    return buf.data
}

fn BodyStream_from_string(s: string) -> BodyStream {
    return BodyStream { data: s, offset: 0 }
}

fn BodyStream_read(stream: BodyStream, max_bytes: i32) -> BodyStream {
    let avail = strlen(stream.data) - stream.offset
    let mut take = max_bytes
    if avail < 0 {
        return BodyStream { data: stream.data, offset: stream.offset }
    }
    if take > avail {
        take = avail
    }
    return BodyStream { data: stream.data, offset: stream.offset + take }
}

fn BodyStream_done(stream: BodyStream) -> i32 {
    if stream.offset >= strlen(stream.data) {
        return 1
    }
    return 0
}

fn BodyStream_remaining(stream: BodyStream) -> i32 {
    let rem = strlen(stream.data) - stream.offset
    if rem < 0 {
        return 0
    }
    return rem
}
