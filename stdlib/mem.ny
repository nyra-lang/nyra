// Built-in memory helpers (implemented in nyra_rt.c).
// Use without import — same as `print` and `time_start`.
//
//   mem_start("alloc")
//   // ... code to measure
//   mem_end("alloc")   // RSS delta in B/KB/MB (green in terminal)
//
//   alloc_track_start("probe")
//   alloc_track_note(64)  // optional byte estimate per allocation site
//   alloc_track_end("probe")

extern fn mem_start(label: string) -> void
extern fn mem_end(label: string) -> void
extern fn alloc_track_start(label: string) -> void
extern fn alloc_track_note(nbytes: i32) -> void
extern fn alloc_track_end(label: string) -> void
