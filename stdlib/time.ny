// Built-in timing helpers (implemented in nyra_rt.c).
// Use without import — same as `print`.
//
//   time_start("loop")
//   // ... code to measure
//   time_end("loop")   // s, ms, µs, ns, ps, fs, or as (green number in terminal)

extern fn time_start(label: string) -> void
extern fn time_end(label: string) -> void

import "time/sugar.ny"
