extern fn strlen(s: &string) -> i32
extern fn strcmp(a: &string, b: &string) -> i32
extern fn substring(s: &string, start: i32, len: i32) -> string

struct GameAudioSession {
    master_volume: f64
    initialized: i32
    current_path: string
}

fn GameAudioSession_new() {
    return GameAudioSession { master_volume: 1.0, initialized: 0, current_path: "" }
}

fn GameAudioSession_set_volume(session, volume) {
    let mut v = volume
    if v < 0.0 {
        v = 0.0
    }
    if v > 1.0 {
        v = 1.0
    }
    return GameAudioSession {
        master_volume: v,
        initialized: session.initialized,
        current_path: session.current_path
    }
}

fn GameAudio_has_suffix(path, suffix) {
    let n = strlen(path)
    let m = strlen(suffix)
    if n < m {
        return 0
    }
    let tail = substring(path, n - m, m)
    if strcmp(tail, suffix) == 0 {
        return 1
    }
    return 0
}

fn GameAudio_is_music_path(path) {
    if GameAudio_has_suffix(path, ".wav") == 1 {
        return 1
    }
    if GameAudio_has_suffix(path, ".ogg") == 1 {
        return 1
    }
    if GameAudio_has_suffix(path, ".mp3") == 1 {
        return 1
    }
    if GameAudio_has_suffix(path, ".flac") == 1 {
        return 1
    }
    return 0
}

fn GameAudio_is_sfx_path(path) {
    if GameAudio_has_suffix(path, ".wav") == 1 {
        return 1
    }
    if GameAudio_has_suffix(path, ".ogg") == 1 {
        return 1
    }
    return 0
}

fn GameAudioSession_path(session: GameAudioSession) {
    return session.current_path
}

fn GameAudioSession_volume(session: GameAudioSession) {
    return session.master_volume
}

fn GameAudioSession_select(session, path) {
    if GameAudio_is_music_path(path) == 0 {
        return session
    }
    return GameAudioSession {
        master_volume: session.master_volume,
        initialized: session.initialized,
        current_path: path
    }
}
