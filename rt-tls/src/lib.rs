//! C ABI for Nyra TLS **client** operations, implemented with rustls.
//!
//! Linked into Nyra user programs as `libnyra_rt_tls.a` — no OpenSSL required for HTTPS.
//! Server listen/accept remain in `stdlib/rt/rt_tls.c` (optional OpenSSL).

#![allow(clippy::missing_safety_doc)]

use std::ffi::{CStr, CString};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::fd::{FromRawFd, RawFd};
use std::ptr;
use std::sync::{Mutex, OnceLock};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{
    ClientConfig, ClientConnection, DigitallySignedStruct, Error as TlsError, RootCertStore,
    SignatureScheme, StreamOwned,
};

const HANDLE_BASE: i32 = 0x100_000;
const MAX_SLOTS: usize = 32;

extern "C" {
    fn rt_tcp_connect(host: *const libc::c_char, port: i32) -> i32;
    /// Optional OpenSSL server-slot closer from `rt_tls.c` (weak / may be unresolved on some links).
    fn rt_tls_server_conn_close(handle: i32);
    fn rt_tls_server_last_error() -> *const libc::c_char;
}

struct TlsSlot {
    used: bool,
    stream: Option<StreamOwned<ClientConnection, TcpStream>>,
}

struct TlsState {
    slots: [TlsSlot; MAX_SLOTS],
    last_error: String,
}

impl TlsState {
    fn new() -> Self {
        Self {
            slots: std::array::from_fn(|_| TlsSlot {
                used: false,
                stream: None,
            }),
            last_error: String::new(),
        }
    }

    fn set_error(&mut self, msg: impl Into<String>) {
        self.last_error = msg.into();
    }

    fn clear_error(&mut self) {
        self.last_error.clear();
    }
}

fn state() -> &'static Mutex<TlsState> {
    static STATE: OnceLock<Mutex<TlsState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(TlsState::new()))
}

fn cstr_to_str<'a>(p: *const libc::c_char) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(p) }.to_str().ok()
}

fn set_err(msg: impl Into<String>) {
    if let Ok(mut st) = state().lock() {
        st.set_error(msg);
    }
}

/// Insecure verifier used only when `verify_peer == 0` (matches OpenSSL `VERIFY_NONE`).
#[derive(Debug)]
struct NoVerifier;

impl ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, TlsError> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn finish_client_config(mut config: ClientConfig) -> ClientConfig {
    // Nyra HTTP client speaks HTTP/1.1 only.
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    config
}

fn build_client_config(ca_path: Option<&str>, verify_peer: bool) -> Result<ClientConfig, String> {
    let provider = std::sync::Arc::new(rustls::crypto::ring::default_provider());

    if !verify_peer {
        let config = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|e| format!("TLS config: {e}"))?
            .dangerous()
            .with_custom_certificate_verifier(std::sync::Arc::new(NoVerifier))
            .with_no_client_auth();
        return Ok(finish_client_config(config));
    }

    let mut roots = RootCertStore::empty();
    if let Some(path) = ca_path.filter(|s| !s.is_empty()) {
        let data = std::fs::read(path).map_err(|e| format!("failed to read CA file: {e}"))?;
        let mut cursor = std::io::Cursor::new(data);
        let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cursor)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("failed to parse CA PEM: {e}"))?;
        if certs.is_empty() {
            return Err("CA file contained no certificates".into());
        }
        for cert in certs {
            roots
                .add(cert)
                .map_err(|e| format!("failed to add CA certificate: {e}"))?;
        }
    } else {
        let native = rustls_native_certs::load_native_certs();
        if !native.errors.is_empty() && native.certs.is_empty() {
            let detail = native
                .errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(format!("failed to load system CA store: {detail}"));
        }
        for cert in native.certs {
            let _ = roots.add(cert);
        }
        if roots.is_empty() {
            return Err("system CA store is empty".into());
        }
    }

    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| format!("TLS config: {e}"))?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(finish_client_config(config))
}

fn handshake_on_fd(
    plain_fd: RawFd,
    hostname: &str,
    ca_path: Option<&str>,
    verify_peer: bool,
) -> Result<StreamOwned<ClientConnection, TcpStream>, String> {
    let config = std::sync::Arc::new(build_client_config(ca_path, verify_peer)?);
    let server_name = ServerName::try_from(hostname.to_string())
        .map_err(|_| format!("invalid TLS hostname: {hostname}"))?;
    let mut conn = ClientConnection::new(config, server_name)
        .map_err(|e| format!("TLS connection setup failed: {e}"))?;
    // Ownership of `plain_fd` moves into TcpStream; drop closes the socket.
    let mut tcp = unsafe { TcpStream::from_raw_fd(plain_fd) };
    while conn.is_handshaking() {
        match conn.complete_io(&mut tcp) {
            Ok((0, 0)) => return Err("TLS handshake failed: unexpected EOF".into()),
            Ok(_) => {}
            Err(e) => {
                let detail = format!("{e}");
                if let Some(src) = std::error::Error::source(&e) {
                    return Err(format!("TLS handshake failed: {detail} ({src})"));
                }
                return Err(format!("TLS handshake failed: {detail}"));
            }
        }
    }
    Ok(StreamOwned::new(conn, tcp))
}

fn alloc_slot(stream: StreamOwned<ClientConnection, TcpStream>) -> Result<i32, String> {
    let mut st = state()
        .lock()
        .map_err(|_| "TLS state lock poisoned".to_string())?;
    for (i, slot) in st.slots.iter_mut().enumerate() {
        if !slot.used {
            slot.used = true;
            slot.stream = Some(stream);
            st.clear_error();
            return Ok(HANDLE_BASE + i as i32);
        }
    }
    Err("TLS handle table full".into())
}

fn with_slot_mut<R>(
    handle: i32,
    f: impl FnOnce(&mut StreamOwned<ClientConnection, TcpStream>) -> R,
) -> Option<R> {
    let mut st = state().lock().ok()?;
    let idx = (handle - HANDLE_BASE) as usize;
    if handle < HANDLE_BASE || idx >= MAX_SLOTS || !st.slots[idx].used {
        return None;
    }
    let stream = st.slots[idx].stream.as_mut()?;
    Some(f(stream))
}

#[no_mangle]
pub extern "C" fn tls_available() -> i32 {
    1
}

#[no_mangle]
pub extern "C" fn rt_tls_last_error() -> *const libc::c_char {
    static ERROR_CSTR: OnceLock<Mutex<CString>> = OnceLock::new();
    let cell = ERROR_CSTR.get_or_init(|| Mutex::new(CString::new("").unwrap()));
    let Ok(st) = state().lock() else {
        return ptr::null();
    };
    let msg = if !st.last_error.is_empty() {
        st.last_error.clone()
    } else {
        let p = unsafe { rt_tls_server_last_error() };
        if p.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(p) }
                .to_string_lossy()
                .into_owned()
        }
    };
    let Ok(mut guard) = cell.lock() else {
        return ptr::null();
    };
    *guard = CString::new(msg.as_str()).unwrap_or_else(|_| CString::new("TLS error").unwrap());
    guard.as_ptr()
}

#[no_mangle]
pub extern "C" fn rt_tls_connect_ex(
    host: *const libc::c_char,
    port: i32,
    ca_path: *const libc::c_char,
    verify_peer: i32,
) -> i32 {
    let Some(host) = cstr_to_str(host) else {
        set_err("invalid host");
        return -1;
    };
    if port <= 0 {
        set_err("invalid host or port");
        return -1;
    }
    let ca = cstr_to_str(ca_path);
    let host_c = match CString::new(host) {
        Ok(c) => c,
        Err(_) => {
            set_err("invalid host");
            return -1;
        }
    };
    let fd = unsafe { rt_tcp_connect(host_c.as_ptr(), port) };
    if fd < 0 {
        set_err("TCP connect failed");
        return -1;
    }
    match handshake_on_fd(fd as RawFd, host, ca, verify_peer != 0) {
        Ok(stream) => match alloc_slot(stream) {
            Ok(h) => h,
            Err(e) => {
                set_err(e);
                -1
            }
        },
        Err(e) => {
            set_err(e);
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn rt_tls_connect(host: *const libc::c_char, port: i32) -> i32 {
    rt_tls_connect_ex(host, port, ptr::null(), 0)
}

#[no_mangle]
pub extern "C" fn rt_tls_connect_verify(host: *const libc::c_char, port: i32) -> i32 {
    rt_tls_connect_ex(host, port, ptr::null(), 1)
}

#[no_mangle]
pub extern "C" fn rt_tls_connect_ca(
    host: *const libc::c_char,
    port: i32,
    ca_path: *const libc::c_char,
) -> i32 {
    rt_tls_connect_ex(host, port, ca_path, 1)
}

#[no_mangle]
pub extern "C" fn rt_tls_upgrade_client_ex(
    plain_fd: i32,
    hostname: *const libc::c_char,
    ca_path: *const libc::c_char,
    verify_peer: i32,
) -> i32 {
    let Some(hostname) = cstr_to_str(hostname) else {
        set_err("invalid fd or hostname");
        return -1;
    };
    if plain_fd < 0 {
        set_err("invalid fd or hostname");
        return -1;
    }
    let ca = cstr_to_str(ca_path);
    match handshake_on_fd(plain_fd as RawFd, hostname, ca, verify_peer != 0) {
        Ok(stream) => match alloc_slot(stream) {
            Ok(h) => h,
            Err(e) => {
                set_err(e);
                -1
            }
        },
        Err(e) => {
            set_err(e);
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn rt_tls_upgrade_client(plain_fd: i32, hostname: *const libc::c_char) -> i32 {
    rt_tls_upgrade_client_ex(plain_fd, hostname, ptr::null(), 0)
}

#[no_mangle]
pub extern "C" fn rt_tls_upgrade_client_verify(
    plain_fd: i32,
    hostname: *const libc::c_char,
) -> i32 {
    rt_tls_upgrade_client_ex(plain_fd, hostname, ptr::null(), 1)
}

#[no_mangle]
pub extern "C" fn rt_tls_read(handle: i32, max_bytes: i32) -> *mut libc::c_char {
    if max_bytes <= 0 {
        return ptr::null_mut();
    }
    let max = max_bytes.clamp(1, 1024 * 1024) as usize;
    let mut buf = vec![0u8; max];
    let n = match with_slot_mut(handle, |stream| stream.read(&mut buf)) {
        Some(Ok(n)) if n > 0 => n,
        _ => return ptr::null_mut(),
    };
    let out = unsafe { libc::malloc(n + 1) as *mut u8 };
    if out.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        ptr::copy_nonoverlapping(buf.as_ptr(), out, n);
        *out.add(n) = 0;
        out as *mut libc::c_char
    }
}

#[no_mangle]
pub extern "C" fn rt_tls_write_bytes(handle: i32, data: *const libc::c_char, len: i32) -> i32 {
    if data.is_null() || len < 0 {
        return -1;
    }
    let slice = unsafe { std::slice::from_raw_parts(data as *const u8, len as usize) };
    match with_slot_mut(handle, |stream| {
        stream.write_all(slice).and_then(|_| stream.flush())
    }) {
        Some(Ok(())) => 0,
        _ => -1,
    }
}

#[no_mangle]
pub extern "C" fn rt_tls_write(handle: i32, data: *const libc::c_char) -> i32 {
    if data.is_null() {
        return -1;
    }
    let len = unsafe { libc::strlen(data) } as i32;
    rt_tls_write_bytes(handle, data, len)
}

#[no_mangle]
pub extern "C" fn rt_tls_read_bytes(handle: i32, buf: *mut libc::c_char, len: i32) -> i32 {
    if buf.is_null() || len <= 0 {
        return -1;
    }
    let out = unsafe { std::slice::from_raw_parts_mut(buf as *mut u8, len as usize) };
    let mut got = 0usize;
    while got < out.len() {
        let n = match with_slot_mut(handle, |stream| stream.read(&mut out[got..])) {
            Some(Ok(0)) => return -1,
            Some(Ok(n)) => n,
            _ => return -1,
        };
        got += n;
    }
    0
}

#[no_mangle]
pub extern "C" fn rt_tls_close(handle: i32) {
    let Ok(mut st) = state().lock() else {
        return;
    };
    let idx = (handle - HANDLE_BASE) as usize;
    if handle < HANDLE_BASE || idx >= MAX_SLOTS || !st.slots[idx].used {
        drop(st);
        // May be an OpenSSL server accept handle owned by rt_tls.c.
        unsafe { rt_tls_server_conn_close(handle) };
        return;
    }
    st.slots[idx].stream = None;
    st.slots[idx].used = false;
}
