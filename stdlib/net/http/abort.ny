// AbortSignal / AbortController — cooperative cancel for HTTP clients.
struct AbortSignal {
    aborted: i32
}

struct AbortController {
    signal: AbortSignal
}

fn AbortSignal_new() -> AbortSignal {
    return AbortSignal { aborted: 0 }
}

fn AbortSignal_aborted(sig: AbortSignal) -> i32 {
    return sig.aborted
}

fn AbortController_new() -> AbortController {
    return AbortController { signal: AbortSignal_new() }
}

fn AbortController_abort(_ctrl: AbortController) -> AbortController {
    return AbortController { signal: AbortSignal { aborted: 1 } }
}

fn AbortController_signal(ctrl: AbortController) -> AbortSignal {
    return ctrl.signal
}
