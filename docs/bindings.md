# Nyra runtime bindings reference

**Generated** by `make gen-bindings-doc` — do not edit by hand.

Stable C symbols live in [`abi-manifest.toml`](abi-manifest.toml) and [`stdlib/nyra_rt.h`](../stdlib/nyra_rt.h).
Nyra stdlib modules declare `extern fn` wrappers that call into the C runtime.

Regenerate:

```bash
make gen-bindings-doc
```

## HashMap runtime naming

Hash-map symbols follow `map_<key_type>_<value_type>_<operation>`. The first type is the **key**, the second is the **value**, then the operation (`new`, `insert`, `get`, `contains`, `remove`, `keys`, `free`, `retain`).

| Family | Example symbols | Use case |
|--------|-----------------|----------|
| `map_str_i32_*` | `map_str_i32_insert`, `map_str_i32_get` | String keys, integer values |
| `map_str_str_*` | `map_str_str_insert`, `map_str_str_get` | String keys, string values |
| `map_i32_i32_*` | `map_i32_i32_insert`, `map_i32_i32_get` | Integer keys and values (`map[int]int` parity) |

When key and value types match, both appear in the name (e.g. `map_i32_i32_get`) so each C entry point has an unambiguous signature. Tutorial: [Learn → HashMap](../webDocs/learn-hashmap.html). Stdlib: `stdlib/map.ny` (`HashMap_str_i32`, `HashMap_str_str`).

## Stable bindings

| Symbol | C signature | RT module | Since | Nyra stdlib |
|--------|-------------|-----------|-------|-------------|
| `aes_cbc_decrypt_hex` | `char *aes_cbc_decrypt_hex(const char *key, const char *ciphertext_hex)` | `rt_aes.c` | 1.3.3 | `stdlib/crypto/aes.ny` |
| `aes_cbc_encrypt_hex` | `char *aes_cbc_encrypt_hex(const char *key, const char *plaintext)` | `rt_aes.c` | 1.3.3 | `stdlib/crypto/aes.ny` |
| `alloc_track_end` | `void alloc_track_end(const char *label)` | `rt_alloc_track.c` | 1.14.0 | `stdlib/mem.ny` |
| `alloc_track_note` | `void alloc_track_note(size_t bytes)` | `rt_alloc_track.c` | 1.14.0 | `stdlib/mem.ny` |
| `alloc_track_start` | `void alloc_track_start(const char *label)` | `rt_alloc_track.c` | 1.14.0 | `stdlib/mem.ny` |
| `ansi_reset` | `const char *ansi_reset(void)` | `rt_io.c` | 1.2.0 | — |
| `append_file` | `int append_file(const char *path, const char *content)` | `rt_fs.c` | 1.1.0 | `stdlib/fs/file.ny` |
| `arc_alloc_i32` | `void *arc_alloc_i32(int value)` | `rt_arc.c` | 2.5.0 | `stdlib/arc.ny` |
| `arc_alloc_string` | `void *arc_alloc_string(const char *value)` | `rt_arc.c` | 2.5.0 | `stdlib/arc.ny` |
| `arc_dec` | `void arc_dec(void *handle)` | `rt_arc.c` | 2.5.0 | — |
| `arc_dec_i32` | `void arc_dec_i32(void *handle)` | `rt_arc.c` | 2.5.0 | `stdlib/arc.ny` |
| `arc_dec_string` | `void arc_dec_string(void *handle)` | `rt_arc.c` | 2.5.0 | `stdlib/arc.ny` |
| `arc_get_i32` | `int arc_get_i32(void *handle)` | `rt_arc.c` | 2.5.0 | `stdlib/arc.ny` |
| `arc_get_string` | `char *arc_get_string(void *handle)` | `rt_arc.c` | 2.5.0 | `stdlib/arc.ny` |
| `arc_inc` | `void arc_inc(void *handle)` | `rt_arc.c` | 2.5.0 | `stdlib/arc.ny` |
| `arena_alloc` | `void *arena_alloc(void *arena, long long nbytes)` | `rt_arena.c` | 1.39.0 | `stdlib/alloc/arena.ny` |
| `arena_free` | `void arena_free(void *arena)` | `rt_arena.c` | 1.39.0 | `stdlib/alloc/arena.ny` |
| `arena_new` | `void *arena_new(long long capacity)` | `rt_arena.c` | 1.39.0 | `stdlib/alloc/arena.ny` |
| `arena_reset` | `void arena_reset(void *arena)` | `rt_arena.c` | 1.39.0 | `stdlib/alloc/arena.ny` |
| `array_bool_debug_string` | `char *array_bool_debug_string(const unsigned char *arr, int n)` | `rt_array.c` | 1.8.0 | — |
| `array_f32_debug_string` | `char *array_f32_debug_string(const float *arr, int n)` | `rt_array.c` | 1.8.0 | — |
| `array_f64_debug_string` | `char *array_f64_debug_string(const double *arr, int n)` | `rt_array.c` | 1.8.0 | — |
| `array_f64_sort_copy` | `void array_f64_sort_copy(double *dst, const double *src, int n)` | `rt_array.c` | 1.3.0 | — |
| `array_i32_debug_string` | `char *array_i32_debug_string(const int *arr, int n)` | `rt_array.c` | 1.8.0 | — |
| `array_i32_sort_copy` | `void array_i32_sort_copy(int *dst, const int *src, int n)` | `rt_array.c` | 1.3.0 | — |
| `array_str_debug_string` | `char *array_str_debug_string(const char *const *arr, int n)` | `rt_array.c` | 1.8.0 | — |
| `async_await` | `int async_await(int handle)` | `rt_async.c` | 0.2.0 | `stdlib/async.ny`, `stdlib/async_v1.ny` |
| `async_await_bool` | `int async_await_bool(int handle)` | `rt_async.c` | 1.22.0 | `stdlib/async/future.ny` |
| `async_await_ptr` | `void *async_await_ptr(int handle)` | `rt_async.c` | 1.22.0 | `stdlib/async/future.ny` |
| `async_future_done` | `int async_future_done(int handle)` | `rt_async.c` | 1.22.0 | `stdlib/async/future.ny` |
| `async_future_ptr_value` | `void *async_future_ptr_value(int handle)` | `rt_async.c` | 1.22.0 | `stdlib/async/future.ny` |
| `async_poll` | `int async_poll(int handle)` | `rt_async.c` | 0.2.0 | `stdlib/async.ny`, `stdlib/async_v1.ny`, `stdlib/net/poll.ny`, `stdlib/net/tcp.ny` |
| `async_poll_bool` | `int async_poll_bool(int handle)` | `rt_async.c` | 1.22.0 | `stdlib/async/future.ny` |
| `async_promise_complete` | `void async_promise_complete(int handle, int value)` | `rt_async.c` | 0.2.0 | `stdlib/async.ny`, `stdlib/async_v1.ny` |
| `async_promise_complete_bool` | `void async_promise_complete_bool(int handle, int value)` | `rt_async.c` | 1.22.0 | `stdlib/async/future.ny` |
| `async_promise_complete_ptr` | `void async_promise_complete_ptr(int handle, void *value)` | `rt_async.c` | 1.22.0 | `stdlib/async/future.ny` |
| `async_promise_new` | `int async_promise_new(void)` | `rt_async.c` | 0.2.0 | `stdlib/async.ny`, `stdlib/async_v1.ny`, `stdlib/net/poll.ny`, `stdlib/os/event_loop.ny` |
| `async_run` | `int async_run(int result)` | `rt_async.c` | 0.2.0 | `stdlib/async.ny` |
| `async_select2_bool` | `int async_select2_bool(int h0, int h1, int *out_index)` | `rt_async.c` | 1.26.0 | — |
| `async_select2_i32` | `int async_select2_i32(int h0, int h1, int *out_index)` | `rt_async.c` | 1.26.0 | — |
| `async_select2_ptr` | `void *async_select2_ptr(int h0, int h1, int *out_index)` | `rt_async.c` | 1.26.0 | — |
| `async_select_i32` | `int async_select_i32(int *handles, int count, int *out_index)` | `rt_async.c` | 1.26.0 | — |
| `async_sleep_ms` | `int async_sleep_ms(int delay_ms)` | `rt_async.c` | 1.4.0 | `stdlib/async_v1.ny` |
| `atan2_f64` | `double atan2_f64(double y, double x)` | `rt_math.c` | 1.16.0 | `stdlib/math.ny` |
| `atomic_add_i32` | `int atomic_add_i32(int *p, int delta)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/atomic.ny` |
| `atomic_cas_i32` | `int atomic_cas_i32(int *p, int expected, int desired)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/atomic.ny` |
| `atomic_i32_free` | `void atomic_i32_free(void *p)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/atomic.ny` |
| `atomic_i32_new` | `void *atomic_i32_new(int initial)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/atomic.ny` |
| `atomic_load_i32` | `int atomic_load_i32(int *p)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/atomic.ny` |
| `atomic_store_i32` | `void atomic_store_i32(int *p, int v)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/atomic.ny` |
| `benchmark_begin` | `void benchmark_begin(void)` | `rt_bench.c` | 1.3.0 | — |
| `benchmark_end` | `void benchmark_end(void)` | `rt_bench.c` | 1.3.0 | — |
| `bin_blob_free` | `void bin_blob_free(void *blob)` | `rt_bin.c` | 2.5.0 | `stdlib/serde/binary.ny` |
| `bin_blob_payload_len` | `int32_t bin_blob_payload_len(void *blob)` | `rt_bin.c` | 2.5.0 | `stdlib/serde/binary.ny` |
| `bin_buf_append_blob` | `void bin_buf_append_blob(void *handle, void *blob)` | `rt_bin.c` | 2.5.0 | `stdlib/serde/binary.ny` |
| `bin_buf_finish` | `void *bin_buf_finish(void *handle)` | `rt_bin.c` | 2.5.0 | `stdlib/serde/binary.ny` |
| `bin_buf_new` | `void *bin_buf_new(void)` | `rt_bin.c` | 2.5.0 | `stdlib/serde/binary.ny` |
| `bin_buf_write_bool` | `void bin_buf_write_bool(void *handle, int flag)` | `rt_bin.c` | 2.5.0 | `stdlib/serde/binary.ny` |
| `bin_buf_write_bytes` | `void bin_buf_write_bytes(void *handle, const void *bytes, int32_t len)` | `rt_bin.c` | 2.5.0 | `stdlib/serde/binary.ny` |
| `bin_buf_write_i32` | `void bin_buf_write_i32(void *handle, int32_t v)` | `rt_bin.c` | 2.5.0 | `stdlib/serde/binary.ny` |
| `bin_buf_write_string` | `void bin_buf_write_string(void *handle, const char *s)` | `rt_bin.c` | 2.5.0 | `stdlib/serde/binary.ny` |
| `bin_decode_blob_at` | `void *bin_decode_blob_at(void *bin, int32_t index)` | `rt_bin.c` | 2.5.0 | `stdlib/serde/binary.ny` |
| `bin_decode_bool_at` | `int32_t bin_decode_bool_at(void *blob, int32_t off)` | `rt_bin.c` | 2.5.0 | `stdlib/serde/binary.ny` |
| `bin_decode_i32_at` | `int32_t bin_decode_i32_at(void *blob, int32_t off)` | `rt_bin.c` | 2.5.0 | `stdlib/serde/binary.ny` |
| `bin_decode_string_at` | `char *bin_decode_string_at(void *blob, int32_t off)` | `rt_bin.c` | 2.5.0 | `stdlib/serde/binary.ny` |
| `bin_field_width_blob_at` | `int32_t bin_field_width_blob_at(void *blob, int32_t off)` | `rt_bin.c` | 2.5.0 | `stdlib/serde/binary.ny` |
| `bin_field_width_bool` | `int32_t bin_field_width_bool(void)` | `rt_bin.c` | 2.5.0 | `stdlib/serde/binary.ny` |
| `bin_field_width_i32` | `int32_t bin_field_width_i32(void)` | `rt_bin.c` | 2.5.0 | `stdlib/serde/binary.ny` |
| `bin_field_width_string_at` | `int32_t bin_field_width_string_at(void *blob, int32_t off)` | `rt_bin.c` | 2.5.0 | `stdlib/serde/binary.ny` |
| `byte_at` | `int byte_at(void *handle, long long index)` | `rt_bytes.c` | 1.3.0 | `stdlib/fs/bytes.ny` |
| `bytes_free` | `void bytes_free(void *handle)` | `rt_bytes.c` | 1.3.0 | `stdlib/fs/bytes.ny` |
| `bytes_from_string` | `void *bytes_from_string(const char *s)` | `rt_bytes.c` | 1.3.0 | `stdlib/fs/bytes.ny` |
| `bytes_len` | `long long bytes_len(void *handle)` | `rt_bytes.c` | 1.3.0 | `stdlib/fs/bytes.ny` |
| `bytes_read_file` | `void *bytes_read_file(const char *path)` | `rt_bytes.c` | 1.3.0 | `stdlib/fs/bytes.ny` |
| `bytes_to_string` | `char *bytes_to_string(void *handle)` | `rt_bytes.c` | 1.3.0 | `stdlib/fs/bytes.ny` |
| `bytes_write_file` | `int bytes_write_file(const char *path, void *handle)` | `rt_bytes.c` | 1.3.0 | `stdlib/fs/bytes.ny` |
| `channel_free` | `void channel_free(void *ch)` | `rt_channel.c` | 0.2.0 | `stdlib/sync/channel.ny` |
| `channel_new` | `void *channel_new(void)` | `rt_channel.c` | 0.2.0 | `stdlib/sync/channel.ny` |
| `channel_recv` | `int channel_recv(void *ch)` | `rt_channel.c` | 0.2.0 | `stdlib/sync/channel.ny` |
| `channel_send` | `void channel_send(void *ch, int value)` | `rt_channel.c` | 0.2.0 | `stdlib/sync/channel.ny` |
| `channel_str_free` | `void channel_str_free(void *ch)` | `rt_channel.c` | 1.12.0 | `stdlib/sync/channel.ny` |
| `channel_str_new` | `void *channel_str_new(void)` | `rt_channel.c` | 1.12.0 | `stdlib/sync/channel.ny` |
| `channel_str_recv` | `char *channel_str_recv(void *ch)` | `rt_channel.c` | 1.12.0 | `stdlib/sync/channel.ny` |
| `channel_str_send` | `void channel_str_send(void *ch, const char *value)` | `rt_channel.c` | 1.12.0 | `stdlib/sync/channel.ny` |
| `char_at` | `int char_at(const char *s, int i)` | `rt_strings.c` | 0.3.0 | `stdlib/gui/buffer.ny`, `stdlib/gui/picker.ny`, `stdlib/gui/syntax.ny`, `stdlib/strings.ny` |
| `color_ansi` | `const char *color_ansi(const char *spec)` | `rt_io.c` | 1.2.0 | — |
| `command_exec_capture` | `char *command_exec_capture(const char *program, void *args_handle)` | `rt_process.c` | 1.14.0 | — |
| `command_run` | `int command_run(const char *program, void *args_handle)` | `rt_process.c` | 1.0.0 | — |
| `copy_dir` | `int copy_dir(const char *src, const char *dst)` | `rt_fs.c` | 1.38.0 | `stdlib/fs/file.ny` |
| `copy_dir_contents` | `int copy_dir_contents(const char *src, const char *dst)` | `rt_fs.c` | 1.38.0 | `stdlib/fs/file.ny` |
| `copy_file` | `long long copy_file(const char *src, const char *dst)` | `rt_fs.c` | 1.0.0 | `stdlib/fs/file.ny` |
| `cos_f64` | `double cos_f64(double x)` | `rt_math.c` | 1.16.0 | `stdlib/math.ny` |
| `cpu_count` | `int32_t cpu_count(void)` | `rt_parallel.c` | 1.3.0 | — |
| `create_dir` | `int create_dir(const char *path)` | `rt_fs.c` | 1.1.0 | `stdlib/fs/file.ny` |
| `create_dir_all` | `int create_dir_all(const char *path)` | `rt_fs.c` | 1.38.0 | `stdlib/fs/file.ny` |
| `date_now` | `void date_now(int *out)` | `rt_time.c` | 1.3.0 | — |
| `error_stack_trace` | `char *error_stack_trace(void)` | `rt_error.c` | 1.40.0 | `stdlib/error.ny` |
| `f64_to_string` | `char *f64_to_string(double n)` | `rt_strings.c` | 1.3.3 | `stdlib/strconv/mod.ny` |
| `file_exists` | `int file_exists(const char *path)` | `rt_fs.c` | 1.1.0 | `stdlib/compress/mod.ny`, `stdlib/fs/file.ny`, `stdlib/gui/picker.ny` |
| `file_size` | `long long file_size(const char *path)` | `rt_fs.c` | 1.0.0 | `stdlib/fs/file.ny` |
| `flate_compress_hex` | `char *flate_compress_hex(const char *data)` | `rt_gzip.c` | 1.3.3 | `stdlib/compress/flate.ny` |
| `flate_decompress_hex` | `char *flate_decompress_hex(const char *hex)` | `rt_gzip.c` | 1.3.3 | `stdlib/compress/flate.ny` |
| `fsync_file` | `int fsync_file(const char *path)` | `rt_fs.c` | 1.18.0 | `stdlib/db/sstable.ny` |
| `gpu_font_draw` | `void gpu_font_draw(const char *text, int x, int y, int font_size, unsigned char r, unsigned char g, unsigned char b, unsigned char a)` | `gpu/rt_gpu_font.c` | 1.3.0 | — |
| `gpu_font_free` | `void gpu_font_free(void)` | `gpu/rt_gpu_font.c` | 1.3.0 | — |
| `gpu_font_init` | `void gpu_font_init(void)` | `gpu/rt_gpu_font.c` | 1.3.0 | — |
| `gunzip_file` | `int gunzip_file(const char *src, const char *dst)` | `rt_gzip.c` | 1.3.0 | `stdlib/compress/gzip.ny` |
| `gzip_compress_hex` | `char *gzip_compress_hex(const char *data)` | `rt_gzip.c` | 1.3.3 | `stdlib/compress/mod.ny` |
| `gzip_decompress_hex` | `char *gzip_decompress_hex(const char *hex)` | `rt_gzip.c` | 1.3.3 | `stdlib/compress/mod.ny` |
| `gzip_file` | `int gzip_file(const char *src, const char *dst)` | `rt_gzip.c` | 1.3.0 | `stdlib/compress/gzip.ny` |
| `heap_free` | `void heap_free(void *p)` | `rt_alloc.c` | 0.2.0 | — |
| `hmac_sha256_hex` | `char *hmac_sha256_hex(const char *key, const char *data)` | `rt_crypto.c` | 1.3.3 | `stdlib/crypto/hmac.ny` |
| `http_download_file` | `int http_download_file(const char *url, const char *path)` | `rt_http.c` | 1.38.0 | `stdlib/http/download.ny` |
| `http_get` | `char *http_get(const char *url)` | `rt_http.c` | 0.3.0 | — |
| `http_status` | `int http_status(const char *response_header)` | `rt_http.c` | 0.3.0 | — |
| `i32_to_string` | `char *i32_to_string(int n)` | `rt_strings.c` | 0.2.0 | `stdlib/bridge/mod.ny`, `stdlib/json/mod.ny`, `stdlib/strconv/mod.ny`, `stdlib/strings.ny`, `stdlib/time/date.ny` |
| `i64_to_string` | `char *i64_to_string(long long n)` | `rt_strings.c` | 1.17.0 | `stdlib/strings.ny` |
| `instant_elapsed_ms` | `int instant_elapsed_ms(int64_t start)` | `rt_time.c` | 1.1.0 | `stdlib/time/instant.ny` |
| `instant_now` | `int64_t instant_now(void)` | `rt_time.c` | 1.1.0 | `stdlib/time/date.ny`, `stdlib/time/instant.ny` |
| `io_pool_create` | `int32_t io_pool_create(int32_t workers)` | `rt_io_pool.c` | 1.39.0 | `stdlib/io/pool.ny` |
| `io_pool_queue_depth` | `int32_t io_pool_queue_depth(int32_t pool)` | `rt_io_pool.c` | 1.39.0 | `stdlib/io/pool.ny` |
| `io_pool_shutdown` | `void io_pool_shutdown(int32_t pool)` | `rt_io_pool.c` | 1.39.0 | `stdlib/io/pool.ny` |
| `io_pool_submit_read` | `int32_t io_pool_submit_read(int32_t pool, int32_t fd, void *buf, int64_t nbytes, int32_t promise)` | `rt_io_pool.c` | 1.39.0 | `stdlib/io/pool.ny` |
| `io_pool_submit_wait_readable` | `int32_t io_pool_submit_wait_readable(int32_t pool, int32_t fd, int32_t promise)` | `rt_io_pool.c` | 1.39.0 | `stdlib/io/pool.ny` |
| `io_register` | `int io_register(int fd, int task_id)` | `rt_async.c` | 0.2.0 | `stdlib/net/poll.ny`, `stdlib/os/event_loop.ny`, `stdlib/terminal/pty.ny` |
| `io_unregister` | `int io_unregister(int fd)` | `rt_async.c` | 1.39.0 | `stdlib/os/event_loop.ny` |
| `io_uring_available` | `int32_t io_uring_available(void)` | `rt_io_uring.c` | 1.39.0 | `stdlib/os/io_uring.ny` |
| `io_uring_pending` | `int io_uring_pending(void)` | `rt_io_uring.c` | 1.39.0 | — |
| `io_uring_register_read` | `int32_t io_uring_register_read(int32_t fd, int32_t promise)` | `rt_io_uring.c` | 1.39.0 | `stdlib/os/io_uring.ny` |
| `io_uring_unregister_read` | `int32_t io_uring_unregister_read(int32_t fd)` | `rt_io_uring.c` | 1.39.0 | `stdlib/os/io_uring.ny` |
| `io_uring_wait_once` | `int io_uring_wait_once(int timeout_ms)` | `rt_io_uring.c` | 1.39.0 | — |
| `io_wait_once` | `int io_wait_once(int timeout_ms)` | `rt_async.c` | 0.2.0 | `stdlib/net/poll.ny` |
| `json_decode_i32_array` | `void *json_decode_i32_array(const char *array_json)` | `rt_json.c` | 1.3.4 | `stdlib/json/mod.ny` |
| `json_decode_ptr_token` | `void *json_decode_ptr_token(const char *json, const char *key)` | `rt_json.c` | 1.9.0 | `stdlib/json/mod.ny` |
| `json_decode_str_array` | `void *json_decode_str_array(const char *array_json)` | `rt_json.c` | 2.5.0 | `stdlib/json/mod.ny` |
| `json_encode_i32_array` | `char *json_encode_i32_array(void *handle)` | `rt_json.c` | 1.3.4 | `stdlib/json/mod.ny` |
| `json_encode_object` | `char *json_encode_object(void *keys_vec, void *values_vec)` | `rt_json.c` | 1.3.4 | `stdlib/json/mod.ny`, `stdlib/serialize/mod.ny` |
| `json_encode_ptr_token` | `char *json_encode_ptr_token(void *p)` | `rt_json.c` | 1.9.0 | `stdlib/json/mod.ny` |
| `json_encode_str_array` | `char *json_encode_str_array(void *handle)` | `rt_json.c` | 2.5.0 | `stdlib/json/mod.ny` |
| `json_get_array` | `char *json_get_array(const char *json, const char *key)` | `rt_json.c` | 1.3.2 | `stdlib/json/mod.ny` |
| `json_get_bool` | `int json_get_bool(const char *json, const char *key)` | `rt_json.c` | 1.3.2 | `stdlib/json/mod.ny` |
| `json_get_i32` | `int json_get_i32(const char *json, const char *key)` | `rt_json.c` | 1.3.2 | `stdlib/json/mod.ny`, `stdlib/process.ny` |
| `json_get_object` | `char *json_get_object(const char *json, const char *key)` | `rt_json.c` | 1.3.2 | `stdlib/json/mod.ny` |
| `json_get_string` | `char *json_get_string(const char *json, const char *key)` | `rt_json.c` | 0.3.0 | `stdlib/bridge/mod.ny`, `stdlib/json/mod.ny`, `stdlib/process.ny` |
| `json_has_bool` | `int json_has_bool(const char *json, const char *key)` | `rt_json.c` | 1.40.0 | `stdlib/json/mod.ny` |
| `json_has_i32` | `int json_has_i32(const char *json, const char *key)` | `rt_json.c` | 1.40.0 | `stdlib/json/mod.ny` |
| `json_has_key` | `int json_has_key(const char *json, const char *key)` | `rt_json.c` | 1.40.0 | `stdlib/json/mod.ny` |
| `json_has_string` | `int json_has_string(const char *json, const char *key)` | `rt_json.c` | 1.40.0 | `stdlib/json/mod.ny` |
| `json_join_raw_array` | `char *json_join_raw_array(void *handle)` | `rt_json.c` | 1.38.0 | `stdlib/json/mod.ny` |
| `json_split_array_elements` | `void *json_split_array_elements(const char *array_json)` | `rt_json.c` | 1.38.0 | `stdlib/json/jsonl.ny`, `stdlib/json/mod.ny` |
| `list_dir` | `char *list_dir(const char *path)` | `rt_fs.c` | 1.3.0 | `stdlib/fs/file.ny`, `stdlib/gui/picker.ny` |
| `map_i32_i32_contains` | `int map_i32_i32_contains(void *handle, int key)` | `rt_map.c` | 1.39.0 | — |
| `map_i32_i32_free` | `void map_i32_i32_free(void *handle)` | `rt_map.c` | 1.39.0 | — |
| `map_i32_i32_get` | `int map_i32_i32_get(void *handle, int key)` | `rt_map.c` | 1.39.0 | — |
| `map_i32_i32_insert` | `void map_i32_i32_insert(void *handle, int key, int value)` | `rt_map.c` | 1.39.0 | — |
| `map_i32_i32_new` | `void *map_i32_i32_new(void)` | `rt_map.c` | 1.39.0 | — |
| `map_i32_i32_retain` | `void map_i32_i32_retain(void *handle)` | `rt_map.c` | 1.39.0 | — |
| `map_str_i32_contains` | `int map_str_i32_contains(void *handle, const char *key)` | `rt_map.c` | 0.4.0 | `stdlib/map.ny` |
| `map_str_i32_free` | `void map_str_i32_free(void *handle)` | `rt_map.c` | 0.4.0 | `stdlib/map.ny` |
| `map_str_i32_get` | `int map_str_i32_get(void *handle, const char *key)` | `rt_map.c` | 0.4.0 | `stdlib/map.ny` |
| `map_str_i32_insert` | `void map_str_i32_insert(void *handle, const char *key, int value)` | `rt_map.c` | 0.4.0 | `stdlib/map.ny` |
| `map_str_i32_keys` | `void *map_str_i32_keys(void *handle)` | `rt_map.c` | 1.14.0 | `stdlib/map.ny` |
| `map_str_i32_new` | `void *map_str_i32_new(void)` | `rt_map.c` | 0.4.0 | `stdlib/map.ny` |
| `map_str_i32_remove` | `int map_str_i32_remove(void *handle, const char *key)` | `rt_map.c` | 1.14.0 | `stdlib/map.ny` |
| `map_str_i32_retain` | `void map_str_i32_retain(void *handle)` | `rt_map.c` | 1.16.0 | `stdlib/map.ny` |
| `map_str_str_contains` | `int map_str_str_contains(void *handle, const char *key)` | `rt_map_str_str.c` | 1.2.0 | `stdlib/map.ny` |
| `map_str_str_free` | `void map_str_str_free(void *handle)` | `rt_map_str_str.c` | 1.2.0 | `stdlib/map.ny` |
| `map_str_str_get` | `const char *map_str_str_get(void *handle, const char *key)` | `rt_map_str_str.c` | 1.2.0 | `stdlib/map.ny` |
| `map_str_str_insert` | `void map_str_str_insert(void *handle, const char *key, const char *value)` | `rt_map_str_str.c` | 1.2.0 | `stdlib/map.ny` |
| `map_str_str_keys` | `void *map_str_str_keys(void *handle)` | `rt_map_str_str.c` | 1.14.0 | `stdlib/map.ny` |
| `map_str_str_new` | `void *map_str_str_new(void)` | `rt_map_str_str.c` | 1.2.0 | `stdlib/map.ny` |
| `map_str_str_remove` | `int map_str_str_remove(void *handle, const char *key)` | `rt_map_str_str.c` | 1.14.0 | `stdlib/map.ny` |
| `map_str_str_retain` | `void map_str_str_retain(void *handle)` | `rt_map_str_str.c` | 1.16.0 | `stdlib/map.ny` |
| `mem_end` | `void mem_end(const char *label)` | `rt_mem.c` | 0.2.0 | `stdlib/mem.ny`, `stdlib/profile/mod.ny` |
| `mem_start` | `void mem_start(const char *label)` | `rt_mem.c` | 0.2.0 | `stdlib/mem.ny`, `stdlib/profile/mod.ny` |
| `mutex_free` | `void mutex_free(void *m)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/mutex.ny` |
| `mutex_lock` | `void mutex_lock(void *m)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/mutex.ny` |
| `mutex_new` | `void *mutex_new(void)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/mutex.ny` |
| `mutex_unlock` | `void mutex_unlock(void *m)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/mutex.ny` |
| `nyra_check_file` | `int nyra_check_file(const char *path)` | `rt_compiler.c` | 1.19.0 | `stdlib/compiler.ny` |
| `nyra_check_source` | `int nyra_check_source(const char *source, const char *file)` | `rt_compiler.c` | 1.19.0 | `stdlib/compiler.ny` |
| `nyra_compiler_free` | `void nyra_compiler_free(char *ptr)` | `rt_compiler.c` | 1.19.0 | `stdlib/compiler.ny` |
| `nyra_diag_json_file` | `char *nyra_diag_json_file(const char *path)` | `rt_compiler.c` | 1.19.0 | `stdlib/compiler.ny` |
| `nyra_diag_json_source` | `char *nyra_diag_json_source(const char *source, const char *file)` | `rt_compiler.c` | 1.19.0 | `stdlib/compiler.ny` |
| `os_arg_at` | `char *os_arg_at(int index)` | `rt_args.c` | 1.3.0 | `stdlib/fs/file.ny` |
| `os_arg_count` | `int os_arg_count(void)` | `rt_args.c` | 1.3.0 | `stdlib/fs/file.ny` |
| `parallel_all_range` | `int32_t parallel_all_range(int32_t start, int32_t end, int32_t (*pred)(int32_t, void *), void *ctx, int32_t max_workers, int32_t exact_workers, int32_t mode, int32_t cpu_percent, int32_t backend)` | `rt_parallel.c` | 1.39.0 | — |
| `parallel_any_range` | `int32_t parallel_any_range(int32_t start, int32_t end, int32_t (*pred)(int32_t, void *), void *ctx, int32_t max_workers, int32_t exact_workers, int32_t mode, int32_t cpu_percent, int32_t backend)` | `rt_parallel.c` | 1.39.0 | — |
| `parallel_find_range` | `int32_t parallel_find_range(int32_t start, int32_t end, int32_t (*pred)(int32_t, void *), void *ctx, int32_t max_workers, int32_t exact_workers, int32_t mode, int32_t cpu_percent, int32_t backend)` | `rt_parallel.c` | 1.39.0 | — |
| `parallel_for_range` | `void parallel_for_range(int32_t start, int32_t end, void (*body)(int32_t, void *), void *ctx, int32_t max_workers, int32_t exact_workers, int32_t mode, int32_t cpu_percent, int32_t backend)` | `rt_parallel.c` | 1.3.0 | — |
| `path_is_dir` | `int path_is_dir(const char *path)` | `rt_fs.c` | 1.3.0 | `stdlib/fs/file.ny`, `stdlib/gui/picker.ny` |
| `println` | `int println(const char *msg)` | `rt_io.c` | 0.2.0 | — |
| `process_exit` | `void process_exit(int code)` | `rt_args.c` | 1.3.0 | `stdlib/flag/mod.ny`, `stdlib/process/exit.ny` |
| `progress_finish` | `void progress_finish(void)` | `rt_progress.c` | 1.3.0 | — |
| `progress_update` | `void progress_update(int32_t current, int32_t total, const char *label)` | `rt_progress.c` | 1.3.0 | — |
| `pty_close` | `void pty_close(int master)` | `rt_pty.c` | 1.3.0 | `stdlib/terminal/pty.ny` |
| `pty_drain` | `char *pty_drain(int master, int max_bytes)` | `rt_pty.c` | 1.3.0 | `stdlib/terminal/pty.ny` |
| `pty_drain_raw` | `char *pty_drain_raw(int master, int max_bytes)` | `rt_pty.c` | 1.3.0 | `stdlib/terminal/pty.ny` |
| `pty_flush_stdout` | `void pty_flush_stdout(int master, int max_bytes, int timeout_ms)` | `rt_pty.c` | 1.3.0 | `stdlib/terminal/pty.ny` |
| `pty_poll` | `int pty_poll(int master)` | `rt_pty.c` | 1.3.0 | `stdlib/terminal/pty.ny` |
| `pty_read` | `char *pty_read(int master, int max_bytes)` | `rt_pty.c` | 1.3.0 | `stdlib/terminal/pty.ny` |
| `pty_read_wait` | `char *pty_read_wait(int master, int max_bytes, int timeout_ms)` | `rt_pty.c` | 1.3.0 | `stdlib/terminal/pty.ny` |
| `pty_read_wait_raw` | `char *pty_read_wait_raw(int master, int max_bytes, int timeout_ms)` | `rt_pty.c` | 1.3.0 | `stdlib/terminal/pty.ny` |
| `pty_resize` | `void pty_resize(int master, int rows, int cols)` | `rt_pty.c` | 1.3.0 | `stdlib/terminal/pty.ny` |
| `pty_spawn` | `int pty_spawn(const char *shell, int rows, int cols)` | `rt_pty.c` | 1.3.0 | `stdlib/terminal/pty.ny` |
| `pty_wait` | `int pty_wait(int master)` | `rt_pty.c` | 1.3.0 | `stdlib/terminal/pty.ny` |
| `pty_write` | `int pty_write(int master, const char *data)` | `rt_pty.c` | 1.3.0 | `stdlib/terminal/pty.ny` |
| `rand_f64` | `double rand_f64(void)` | `rt_random.c` | 1.16.0 | `stdlib/builtins_math.ny` |
| `rand_f64_range` | `double rand_f64_range(double min_val, double max_val)` | `rt_random.c` | 1.39.0 | — |
| `rand_i32` | `int rand_i32(void)` | `rt_random.c` | 1.1.0 | — |
| `rand_i64` | `int64_t rand_i64(void)` | `rt_random.c` | 1.39.0 | — |
| `rand_range` | `int rand_range(int min_val, int max_val)` | `rt_random.c` | 1.1.0 | `stdlib/random.ny` |
| `rand_range_i64` | `int64_t rand_range_i64(int64_t min_val, int64_t max_val)` | `rt_random.c` | 1.39.0 | — |
| `rand_range_u32` | `uint32_t rand_range_u32(uint32_t min_val, uint32_t max_val)` | `rt_random.c` | 1.39.0 | — |
| `rand_range_u64` | `uint64_t rand_range_u64(uint64_t min_val, uint64_t max_val)` | `rt_random.c` | 1.39.0 | — |
| `rand_u32` | `uint32_t rand_u32(void)` | `rt_random.c` | 1.39.0 | — |
| `rand_u64` | `uint64_t rand_u64(void)` | `rt_random.c` | 1.39.0 | — |
| `random_hex` | `char *random_hex(int byte_count)` | `rt_random.c` | 1.1.0 | `stdlib/crypto/random.ny`, `stdlib/uuid/mod.ny` |
| `read_file` | `char *read_file(const char *path)` | `rt_fs.c` | 0.2.0 | `stdlib/compress/mod.ny`, `stdlib/fs/file.ny` |
| `read_file_limit` | `char *read_file_limit(const char *path, int max_bytes)` | `rt_fs.c` | 1.13.0 | `stdlib/fs/file.ny` |
| `regex_compile` | `void *regex_compile(const char *pattern)` | `rt_regex.c` | 1.3.0 | `stdlib/strings/regex.ny` |
| `regex_free` | `void regex_free(void *handle)` | `rt_regex.c` | 1.3.0 | `stdlib/strings/regex.ny` |
| `regex_is_match` | `int regex_is_match(void *handle, const char *text)` | `rt_regex.c` | 1.3.0 | `stdlib/strings/regex.ny` |
| `remove_dir` | `int remove_dir(const char *path)` | `rt_fs.c` | 1.1.0 | `stdlib/fs/file.ny` |
| `remove_dir_all` | `int remove_dir_all(const char *path)` | `rt_fs.c` | 1.38.0 | `stdlib/fs/file.ny` |
| `remove_file` | `int remove_file(const char *path)` | `rt_fs.c` | 1.1.0 | `stdlib/fs/file.ny` |
| `rsa_available` | `int rsa_available(void)` | `rt_crypto_openssl.c` | 1.3.3 | `stdlib/crypto/rsa.ny` |
| `rsa_public_encrypt_pem` | `char *rsa_public_encrypt_pem(const char *pem_pub, const char *plaintext)` | `rt_crypto_openssl.c` | 1.3.3 | `stdlib/crypto/rsa.ny` |
| `rsa_sha256_sign_pem` | `char *rsa_sha256_sign_pem(const char *pem_priv, const char *message)` | `rt_crypto_openssl.c` | 1.3.3 | `stdlib/crypto/rsa.ny` |
| `rt_args_init` | `void rt_args_init(int argc, char **argv)` | `rt_args.c` | 1.3.0 | — |
| `rt_bridge_exec` | `char *rt_bridge_exec(const char *program, const char *input)` | `rt_process.c` | 1.2.0 | `stdlib/bridge/mod.ny` |
| `rt_bridge_exec_arg` | `char *rt_bridge_exec_arg(const char *program, const char *arg1, const char *input)` | `rt_process.c` | 1.2.0 | `stdlib/bridge/mod.ny` |
| `rt_dns_lookup` | `char *rt_dns_lookup(const char *host)` | `rt_net.c` | 1.11.0 | `stdlib/net/dns.ny` |
| `rt_icmp_capable` | `int rt_icmp_capable(void)` | `rt_net.c` | 1.20.0 | `stdlib/net/icmp.ny` |
| `rt_icmp_ping_ms` | `int rt_icmp_ping_ms(const char *host, int timeout_ms)` | `rt_net.c` | 1.12.0 | `stdlib/net/icmp.ny` |
| `rt_icmp_ping_system_ms` | `int rt_icmp_ping_system_ms(const char *host, int timeout_ms)` | `rt_net.c` | 1.20.0 | `stdlib/net/icmp.ny` |
| `rt_tcp_accept` | `int rt_tcp_accept(int listener_fd)` | `rt_net.c` | 0.3.0 | — |
| `rt_tcp_accept_async` | `int rt_tcp_accept_async(int listener_fd)` | `rt_net.c` | 0.3.0 | — |
| `rt_tcp_close` | `void rt_tcp_close(int fd)` | `rt_net.c` | 0.3.0 | — |
| `rt_tcp_connect` | `int rt_tcp_connect(const char *host, int port)` | `rt_net.c` | 0.3.0 | — |
| `rt_tcp_connect_timeout` | `int rt_tcp_connect_timeout(const char *host, int port, int timeout_ms)` | `rt_net.c` | 1.11.0 | `stdlib/net/tcp.ny` |
| `rt_tcp_hub_add` | `int32_t rt_tcp_hub_add(void *hub, int32_t fd)` | `rt_tcp_hub.c` | 1.12.0 | `stdlib/net/hub.ny` |
| `rt_tcp_hub_broadcast` | `void rt_tcp_hub_broadcast(void *hub, const char *msg)` | `rt_tcp_hub.c` | 1.12.0 | `stdlib/net/hub.ny` |
| `rt_tcp_hub_free` | `void rt_tcp_hub_free(void *hub)` | `rt_tcp_hub.c` | 1.12.0 | `stdlib/net/hub.ny` |
| `rt_tcp_hub_new` | `void *rt_tcp_hub_new(int32_t max_clients)` | `rt_tcp_hub.c` | 1.12.0 | `stdlib/net/hub.ny` |
| `rt_tcp_hub_remove` | `void rt_tcp_hub_remove(void *hub, int32_t fd)` | `rt_tcp_hub.c` | 1.12.0 | `stdlib/net/hub.ny` |
| `rt_tcp_listen` | `int rt_tcp_listen(const char *host, int port)` | `rt_net.c` | 0.3.0 | — |
| `rt_tcp_ping_ms` | `int rt_tcp_ping_ms(const char *host, int port, int timeout_ms)` | `rt_net.c` | 1.11.0 | `stdlib/net/icmp.ny` |
| `rt_tcp_read` | `char *rt_tcp_read(int fd, int max_bytes)` | `rt_net.c` | 0.3.0 | — |
| `rt_tcp_write` | `int rt_tcp_write(int fd, const char *data)` | `rt_net.c` | 0.3.0 | — |
| `rt_tls_accept` | `int rt_tls_accept(int listener_handle)` | `rt_tls.c` | 1.3.3 | `stdlib/tls.ny` |
| `rt_tls_close` | `void rt_tls_close(int handle)` | `rt_tls.c` | 1.0.0 | `stdlib/tls.ny` |
| `rt_tls_connect` | `int rt_tls_connect(const char *host, int port)` | `rt_tls.c` | 1.0.0 | `stdlib/tls.ny` |
| `rt_tls_connect_ca` | `int rt_tls_connect_ca(const char *host, int port, const char *ca_path)` | `rt_tls.c` | 1.20.0 | `stdlib/tls.ny` |
| `rt_tls_connect_ex` | `int rt_tls_connect_ex(const char *host, int port, const char *ca_path, int verify_peer)` | `rt_tls.c` | 2.5.0 | — |
| `rt_tls_connect_verify` | `int rt_tls_connect_verify(const char *host, int port)` | `rt_tls.c` | 2.5.0 | `stdlib/tls.ny` |
| `rt_tls_gen_self_signed` | `int rt_tls_gen_self_signed(const char *cert_path, const char *key_path, const char *common_name)` | `rt_tls.c` | 1.15.0 | `stdlib/net/tls_dev.ny` |
| `rt_tls_last_error` | `const char *rt_tls_last_error(void)` | `rt_tls.c` | 1.20.0 | `stdlib/tls.ny` |
| `rt_tls_listen` | `int rt_tls_listen(const char *cert_pem_path, const char *key_pem_path, const char *host, int port)` | `rt_tls.c` | 1.3.3 | `stdlib/tls.ny` |
| `rt_tls_listener_close` | `void rt_tls_listener_close(int listener_handle)` | `rt_tls.c` | 1.3.3 | `stdlib/tls.ny` |
| `rt_tls_read` | `char *rt_tls_read(int handle, int max_bytes)` | `rt_tls.c` | 1.0.0 | `stdlib/tls.ny` |
| `rt_tls_upgrade_client` | `int rt_tls_upgrade_client(int plain_fd, const char *hostname)` | `rt_tls.c` | 1.14.0 | `stdlib/tls.ny` |
| `rt_tls_upgrade_client_ex` | `int rt_tls_upgrade_client_ex(int plain_fd, const char *hostname, const char *ca_path, int verify_peer)` | `rt_tls.c` | 1.20.0 | `stdlib/tls.ny` |
| `rt_tls_upgrade_client_verify` | `int rt_tls_upgrade_client_verify(int plain_fd, const char *hostname)` | `rt_tls.c` | 2.5.0 | `stdlib/tls.ny` |
| `rt_tls_validate_pem_files` | `int rt_tls_validate_pem_files(const char *cert_pem_path, const char *key_pem_path)` | `rt_tls.c` | 1.20.0 | `stdlib/tls.ny` |
| `rt_tls_write` | `int rt_tls_write(int handle, const char *data)` | `rt_tls.c` | 1.0.0 | `stdlib/tls.ny` |
| `rt_udp_bind` | `int rt_udp_bind(const char *host, int port)` | `rt_net.c` | 1.3.3 | `stdlib/net/udp.ny` |
| `rt_udp_close` | `void rt_udp_close(int fd)` | `rt_net.c` | 1.3.3 | `stdlib/net/udp.ny` |
| `rt_udp_recv` | `char *rt_udp_recv(int fd, int max_bytes)` | `rt_net.c` | 1.3.3 | `stdlib/net/udp.ny` |
| `rt_udp_send` | `int rt_udp_send(int fd, const char *host, int port, const char *data)` | `rt_net.c` | 1.3.3 | `stdlib/net/udp.ny` |
| `runtime_executor_run_until` | `int runtime_executor_run_until(int handle, int timeout_ms)` | `rt_async.c` | 1.4.0 | `stdlib/async_v1.ny` |
| `runtime_executor_tick` | `int runtime_executor_tick(int timeout_ms)` | `rt_async.c` | 1.4.0 | `stdlib/async_v1.ny` |
| `runtime_poll_io` | `int runtime_poll_io(int timeout_ms)` | `rt_async.c` | 1.4.0 | `stdlib/async_v1.ny` |
| `runtime_run` | `void runtime_run(void)` | `rt_async.c` | 0.2.0 | `stdlib/async.ny`, `stdlib/async_v1.ny` |
| `rwlock_free` | `void rwlock_free(void *r)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/rwlock.ny` |
| `rwlock_new` | `void *rwlock_new(void)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/rwlock.ny` |
| `rwlock_rlock` | `void rwlock_rlock(void *r)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/rwlock.ny` |
| `rwlock_unlock` | `void rwlock_unlock(void *r)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/rwlock.ny` |
| `rwlock_wlock` | `void rwlock_wlock(void *r)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/rwlock.ny` |
| `sha256_hex` | `char *sha256_hex(const char *data)` | `rt_crypto.c` | 1.3.2 | `stdlib/crypto/sha256.ny` |
| `sha512_hex` | `char *sha512_hex(const char *data)` | `rt_crypto_sha512.c` | 1.3.3 | `stdlib/crypto/sha512.ny` |
| `shm_close_fd` | `int32_t shm_close_fd(int32_t fd)` | `rt_shm.c` | 1.39.0 | `stdlib/os/shm.ny` |
| `shm_create` | `int32_t shm_create(const char *name, int64_t nbytes)` | `rt_shm.c` | 1.39.0 | `stdlib/os/shm.ny` |
| `shm_map` | `void *shm_map(int32_t fd, int64_t nbytes)` | `rt_shm.c` | 1.39.0 | `stdlib/os/shm.ny` |
| `shm_open_existing` | `int32_t shm_open_existing(const char *name, int64_t nbytes)` | `rt_shm.c` | 1.39.0 | `stdlib/os/shm.ny` |
| `shm_unlink_region` | `int32_t shm_unlink_region(const char *name)` | `rt_shm.c` | 1.39.0 | `stdlib/os/shm.ny` |
| `shm_unmap` | `int32_t shm_unmap(void *addr, int64_t nbytes)` | `rt_shm.c` | 1.39.0 | `stdlib/os/shm.ny` |
| `sin_f64` | `double sin_f64(double x)` | `rt_math.c` | 1.16.0 | `stdlib/math.ny` |
| `sleep_ms` | `void sleep_ms(int ms)` | `rt_time.c` | 1.1.0 | `stdlib/time/instant.ny` |
| `spawn` | `void spawn(void)` | `rt_async.c` | 0.2.0 | — |
| `spawn_capture` | `void *spawn_capture(void (*body)(void *), void *data, int64_t nbytes)` | `rt_spawn.c` | 0.2.0 | — |
| `spawn_handle_drop` | `void spawn_handle_drop(void *handle)` | `rt_spawn.c` | 1.39.0 | — |
| `spawn_join` | `int spawn_join(void *handle)` | `rt_spawn.c` | 1.39.0 | — |
| `spawn_task_capture` | `void *spawn_task_capture(void (*body)(void *), void *data, int64_t nbytes)` | `rt_task_pool.c` | 1.39.0 | — |
| `spawn_task_handle_drop` | `void spawn_task_handle_drop(void *handle)` | `rt_task_pool.c` | 1.39.0 | — |
| `spawn_task_join` | `int spawn_task_join(void *handle)` | `rt_task_pool.c` | 1.39.0 | — |
| `sqlite_close` | `void sqlite_close(void *handle)` | `rt_sqlite.c` | 1.3.2 | `stdlib/db/sql.ny`, `stdlib/db/sqlite.ny` |
| `sqlite_column_count` | `int sqlite_column_count(void *stmt)` | `rt_sqlite.c` | 1.21.0 | `stdlib/db/sqlite.ny` |
| `sqlite_column_text` | `const char *sqlite_column_text(void *stmt, int col)` | `rt_sqlite.c` | 1.21.0 | `stdlib/db/sqlite.ny` |
| `sqlite_exec` | `int sqlite_exec(void *handle, const char *sql)` | `rt_sqlite.c` | 1.3.2 | `stdlib/db/sql.ny`, `stdlib/db/sqlite.ny` |
| `sqlite_finalize` | `void sqlite_finalize(void *stmt)` | `rt_sqlite.c` | 1.21.0 | `stdlib/db/sqlite.ny` |
| `sqlite_last_error` | `const char *sqlite_last_error(void *handle)` | `rt_sqlite.c` | 1.21.0 | `stdlib/db/sqlite.ny` |
| `sqlite_open` | `void *sqlite_open(const char *path)` | `rt_sqlite.c` | 1.3.2 | `stdlib/db/sqlite.ny` |
| `sqlite_prepare` | `void *sqlite_prepare(void *handle, const char *sql)` | `rt_sqlite.c` | 1.21.0 | `stdlib/db/sqlite.ny` |
| `sqlite_query_rows` | `void *sqlite_query_rows(void *handle, const char *sql)` | `rt_sqlite.c` | 1.18.0 | `stdlib/db/sql.ny`, `stdlib/db/sqlite.ny` |
| `sqlite_rowset_at` | `const char *sqlite_rowset_at(void *rowset, int row, int col)` | `rt_sqlite.c` | 1.18.0 | `stdlib/db/sqlite.ny` |
| `sqlite_rowset_cols` | `int sqlite_rowset_cols(void *rowset)` | `rt_sqlite.c` | 1.18.0 | `stdlib/db/sqlite.ny` |
| `sqlite_rowset_free` | `void sqlite_rowset_free(void *rowset)` | `rt_sqlite.c` | 1.18.0 | `stdlib/db/sqlite.ny` |
| `sqlite_rowset_rows` | `int sqlite_rowset_rows(void *rowset)` | `rt_sqlite.c` | 1.18.0 | `stdlib/db/sqlite.ny` |
| `sqlite_step` | `int sqlite_step(void *stmt)` | `rt_sqlite.c` | 1.21.0 | `stdlib/db/sqlite.ny` |
| `stdin_read_bytes` | `void *stdin_read_bytes(int max_bytes)` | `rt_bytes.c` | 1.3.0 | `stdlib/fs/bytes.ny` |
| `stdin_read_key` | `int stdin_read_key(void)` | `rt_io.c` | 1.16.0 | `stdlib/terminal/raw.ny` |
| `stdin_read_line` | `char *stdin_read_line(const char *prompt)` | `rt_io.c` | 1.1.0 | `stdlib/bufio/mod.ny`, `stdlib/terminal/mod.ny` |
| `stdin_set_raw_mode` | `void stdin_set_raw_mode(int enable)` | `rt_io.c` | 1.16.0 | `stdlib/terminal/raw.ny` |
| `stdout_flush` | `void stdout_flush(void)` | `rt_io.c` | 0.2.0 | — |
| `stdout_write_bytes` | `void stdout_write_bytes(void *handle)` | `rt_bytes.c` | 1.3.0 | `stdlib/fs/bytes.ny` |
| `stdout_write_i32` | `void stdout_write_i32(int n)` | `rt_io.c` | 0.2.0 | — |
| `stdout_write_str` | `void stdout_write_str(const char *s)` | `rt_io.c` | 0.2.0 | `stdlib/terminal/mod.ny` |
| `stdout_writeln_i32` | `void stdout_writeln_i32(int n)` | `rt_io.c` | 0.2.0 | — |
| `stdout_writeln_str` | `void stdout_writeln_str(const char *s)` | `rt_io.c` | 0.2.0 | `stdlib/log.ny`, `stdlib/terminal/mod.ny` |
| `str_buf_append` | `void str_buf_append(void *handle, const char *piece)` | `rt_str_buf.c` | 1.39.0 | `stdlib/strings/builder.ny` |
| `str_buf_append_char` | `void str_buf_append_char(void *handle, int ch)` | `rt_str_buf.c` | 1.39.0 | `stdlib/strings/builder.ny` |
| `str_buf_build` | `char *str_buf_build(void *handle)` | `rt_str_buf.c` | 1.39.0 | `stdlib/strings/builder.ny` |
| `str_buf_drop` | `void str_buf_drop(void *handle)` | `rt_str_buf.c` | 1.39.0 | `stdlib/strings/builder.ny` |
| `str_buf_new` | `void *str_buf_new(void)` | `rt_str_buf.c` | 1.39.0 | `stdlib/strings/builder.ny` |
| `str_cat` | `char *str_cat(const char *a, const char *b)` | `rt_strings.c` | 0.2.0 | — |
| `str_clone` | `char *str_clone(const char *s)` | `rt_alloc.c` | 3.0.0 | — |
| `str_cmp` | `int str_cmp(const char *a, const char *b)` | `rt_strings.c` | 0.3.0 | — |
| `str_contains` | `int str_contains(const char *hay, const char *needle)` | `rt_strings.c` | 1.3.0 | `stdlib/strings/ops.ny` |
| `str_ends_with` | `int str_ends_with(const char *s, const char *suffix)` | `rt_strings.c` | 1.3.0 | `stdlib/strings/ops.ny` |
| `str_len` | `int str_len(const char *s)` | `rt_strings.c` | 0.2.0 | `stdlib/strings/ops.ny` |
| `str_pop` | `char *str_pop(const char *s)` | `rt_strings.c` | 1.3.0 | `stdlib/gui/buffer.ny`, `stdlib/strings.ny` |
| `str_push_char` | `char *str_push_char(const char *s, int ch)` | `rt_strings.c` | 1.3.0 | `stdlib/gui/buffer.ny`, `stdlib/strings.ny` |
| `str_replace` | `char *str_replace(const char *s, const char *from, const char *to)` | `rt_strings.c` | 1.3.0 | `stdlib/strings/ops.ny` |
| `str_replacen` | `char *str_replacen(const char *s, const char *from, const char *to, int count)` | `rt_strings.c` | 1.5.0 | `stdlib/strings/ops.ny` |
| `str_split` | `void *str_split(const char *s, const char *sep)` | `rt_strings.c` | 1.3.0 | `stdlib/builtins_string.ny` |
| `str_starts_with` | `int str_starts_with(const char *s, const char *prefix)` | `rt_strings.c` | 1.3.0 | `stdlib/strings/ops.ny` |
| `str_strip_ansi` | `char *str_strip_ansi(const char *input)` | `rt_strings.c` | 1.3.0 | — |
| `str_strip_suffix` | `char *str_strip_suffix(const char *s, const char *suffix)` | `rt_strings.c` | 1.0.0 | `stdlib/strings.ny` |
| `str_to_camel_case` | `char *str_to_camel_case(const char *s)` | `rt_strings.c` | 1.40.3 | `stdlib/strings.ny` |
| `str_to_capitalize` | `char *str_to_capitalize(const char *s)` | `rt_strings.c` | 1.40.3 | `stdlib/strings.ny` |
| `str_to_dot_case` | `char *str_to_dot_case(const char *s)` | `rt_strings.c` | 1.40.3 | `stdlib/strings.ny` |
| `str_to_f64` | `double str_to_f64(const char *s)` | `rt_strings.c` | 1.3.3 | `stdlib/strconv/mod.ny` |
| `str_to_i32` | `int str_to_i32(const char *s)` | `rt_strings.c` | 1.3.0 | `stdlib/strconv/mod.ny`, `stdlib/strings.ny` |
| `str_to_kebab_case` | `char *str_to_kebab_case(const char *s)` | `rt_strings.c` | 1.40.3 | `stdlib/strings.ny` |
| `str_to_lower` | `char *str_to_lower(const char *s)` | `rt_strings.c` | 1.1.0 | `stdlib/strings/ops.ny` |
| `str_to_lowercase` | `char *str_to_lowercase(const char *s)` | `rt_strings.c` | 1.40.3 | `stdlib/strings.ny` |
| `str_to_pascal_case` | `char *str_to_pascal_case(const char *s)` | `rt_strings.c` | 1.40.3 | `stdlib/strings.ny` |
| `str_to_screaming_snake_case` | `char *str_to_screaming_snake_case(const char *s)` | `rt_strings.c` | 1.40.3 | `stdlib/strings.ny` |
| `str_to_snake_case` | `char *str_to_snake_case(const char *s)` | `rt_strings.c` | 1.40.3 | `stdlib/strings.ny` |
| `str_to_titlecase` | `char *str_to_titlecase(const char *s)` | `rt_strings.c` | 1.40.3 | `stdlib/strings.ny` |
| `str_to_train_case` | `char *str_to_train_case(const char *s)` | `rt_strings.c` | 1.40.3 | `stdlib/strings.ny` |
| `str_to_upper` | `char *str_to_upper(const char *s)` | `rt_strings.c` | 1.1.0 | `stdlib/strings/ops.ny` |
| `str_trim` | `char *str_trim(const char *s)` | `rt_strings.c` | 1.1.0 | `stdlib/strings/ops.ny` |
| `strstr_pos` | `int strstr_pos(const char *hay, const char *needle)` | `rt_strings.c` | 0.3.0 | `stdlib/config/mod.ny`, `stdlib/gui/picker.ny`, `stdlib/gui/syntax.ny`, `stdlib/strings.ny` |
| `substring` | `char *substring(const char *s, int start, int len)` | `rt_strings.c` | 0.3.0 | `stdlib/games/audio.ny`, `stdlib/gui/buffer.ny`, `stdlib/gui/picker.ny`, `stdlib/gui/syntax.ny`, `stdlib/strings.ny`, `stdlib/uuid/mod.ny` |
| `sys_accept` | `int sys_accept(int listener_fd)` | `rt_net.c` | 0.3.0 | `stdlib/net/syscall.ny` |
| `sys_close` | `void sys_close(int fd)` | `rt_net.c` | 0.3.0 | `stdlib/net/syscall.ny` |
| `sys_connect` | `int sys_connect(const char *host, int port)` | `rt_net.c` | 0.3.0 | `stdlib/net/syscall.ny` |
| `sys_listen` | `int sys_listen(const char *host, int port)` | `rt_net.c` | 0.3.0 | `stdlib/net/syscall.ny` |
| `sys_recv` | `char *sys_recv(int fd, int max_bytes)` | `rt_net.c` | 0.3.0 | `stdlib/net/syscall.ny` |
| `sys_send` | `int sys_send(int fd, const char *data)` | `rt_net.c` | 0.3.0 | `stdlib/net/syscall.ny` |
| `sys_set_nonblock` | `int sys_set_nonblock(int fd)` | `rt_net.c` | 0.3.0 | `stdlib/net/syscall.ny` |
| `tan_f64` | `double tan_f64(double x)` | `rt_math.c` | 1.16.0 | `stdlib/math.ny` |
| `tar_create` | `int tar_create(const char *archive, void *paths_vec)` | `rt_tar.c` | 1.3.0 | `stdlib/archive/tar.ny` |
| `tar_extract` | `int tar_extract(const char *archive, const char *out_dir)` | `rt_tar.c` | 1.3.0 | `stdlib/archive/tar.ny` |
| `time_end` | `void time_end(const char *label)` | `rt_time.c` | 0.2.0 | `stdlib/bench/mod.ny`, `stdlib/profile/mod.ny`, `stdlib/time.ny` |
| `time_start` | `void time_start(const char *label)` | `rt_time.c` | 0.2.0 | `stdlib/bench/mod.ny`, `stdlib/profile/mod.ny`, `stdlib/time.ny` |
| `tls_available` | `int tls_available(void)` | `rt_tls.c` | 1.0.0 | `stdlib/tls.ny` |
| `vec_bytes_free` | `void vec_bytes_free(void *handle)` | `rt_vec.c` | 1.17.0 | `stdlib/collections/vec_pod.ny` |
| `vec_bytes_get` | `void vec_bytes_get(void *handle, int index, void *out)` | `rt_vec.c` | 1.17.0 | `stdlib/collections/vec_pod.ny` |
| `vec_bytes_get_ptr` | `void *vec_bytes_get_ptr(void *handle, int index)` | `rt_vec.c` | 1.25.0 | `stdlib/collections/vec_pod.ny` |
| `vec_bytes_len` | `int vec_bytes_len(void *handle)` | `rt_vec.c` | 1.17.0 | `stdlib/collections/vec_pod.ny` |
| `vec_bytes_new` | `void *vec_bytes_new(int elem_size)` | `rt_vec.c` | 1.17.0 | `stdlib/collections/vec_pod.ny` |
| `vec_bytes_push` | `void vec_bytes_push(void *handle, void *elem)` | `rt_vec.c` | 1.17.0 | `stdlib/collections/vec_pod.ny` |
| `vec_bytes_push_ptr` | `void vec_bytes_push_ptr(void *handle, void *elem)` | `rt_vec.c` | 1.25.0 | `stdlib/collections/vec_pod.ny` |
| `vec_i32_free` | `void vec_i32_free(void *handle)` | `rt_vec.c` | 0.4.0 | `stdlib/vec.ny` |
| `vec_i32_get` | `int vec_i32_get(void *handle, int index)` | `rt_vec.c` | 0.4.0 | `stdlib/vec.ny` |
| `vec_i32_len` | `int vec_i32_len(void *handle)` | `rt_vec.c` | 0.4.0 | `stdlib/vec.ny` |
| `vec_i32_new` | `void *vec_i32_new(void)` | `rt_vec.c` | 0.4.0 | `stdlib/vec.ny` |
| `vec_i32_pop` | `int vec_i32_pop(void *handle)` | `rt_vec.c` | 1.1.0 | `stdlib/vec.ny` |
| `vec_i32_push` | `void vec_i32_push(void *handle, int value)` | `rt_vec.c` | 0.4.0 | `stdlib/vec.ny` |
| `vec_i32_set` | `void vec_i32_set(void *handle, int index, int value)` | `rt_vec.c` | 1.20.0 | `stdlib/vec.ny` |
| `vec_str_free` | `void vec_str_free(void *handle)` | `rt_vec.c` | 1.3.0 | `stdlib/vec_str.ny` |
| `vec_str_from_argv` | `void *vec_str_from_argv(int start_index)` | `rt_args.c` | 1.3.0 | `stdlib/vec_str.ny` |
| `vec_str_get` | `const char *vec_str_get(void *handle, int index)` | `rt_vec.c` | 1.3.0 | `stdlib/vec_str.ny` |
| `vec_str_len` | `int vec_str_len(void *handle)` | `rt_vec.c` | 1.3.0 | `stdlib/vec_str.ny` |
| `vec_str_new` | `void *vec_str_new(void)` | `rt_vec.c` | 1.3.0 | `stdlib/vec_str.ny` |
| `vec_str_push` | `void vec_str_push(void *handle, const char *value)` | `rt_vec.c` | 1.3.0 | `stdlib/vec_str.ny` |
| `waitgroup_add` | `void waitgroup_add(void *wg, int delta)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/waitgroup.ny` |
| `waitgroup_done` | `void waitgroup_done(void *wg)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/waitgroup.ny` |
| `waitgroup_free` | `void waitgroup_free(void *wg)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/waitgroup.ny` |
| `waitgroup_new` | `void *waitgroup_new(void)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/waitgroup.ny` |
| `waitgroup_wait` | `void waitgroup_wait(void *wg)` | `rt_sync.c` | 1.3.3 | `stdlib/sync/waitgroup.ny` |
| `write_file` | `int write_file(const char *path, const char *content)` | `rt_fs.c` | 0.2.0 | `stdlib/compress/mod.ny`, `stdlib/fs/file.ny` |
| `ws_accept_handshake` | `int ws_accept_handshake(int listener_fd)` | `rt_websocket.c` | 1.11.0 | `stdlib/net/websocket.ny` |
| `ws_accept_tls_handshake` | `int ws_accept_tls_handshake(int tls_listener_handle)` | `rt_websocket.c` | 1.14.0 | `stdlib/net/websocket.ny` |
| `ws_close` | `void ws_close(int fd)` | `rt_websocket.c` | 1.3.3 | `stdlib/net/websocket.ny` |
| `ws_connect` | `int ws_connect(const char *url)` | `rt_websocket.c` | 1.3.3 | `stdlib/net/websocket.ny` |
| `ws_listen` | `int ws_listen(const char *host, int port)` | `rt_websocket.c` | 1.11.0 | `stdlib/net/websocket.ny` |
| `ws_listen_tls` | `int ws_listen_tls(const char *cert_path, const char *key_path, const char *host, int port)` | `rt_websocket.c` | 1.14.0 | `stdlib/net/websocket.ny` |
| `ws_recv_text` | `char *ws_recv_text(int fd, int max_bytes)` | `rt_websocket.c` | 1.3.3 | `stdlib/net/websocket.ny` |
| `ws_send_text` | `int ws_send_text(int fd, const char *text)` | `rt_websocket.c` | 1.3.3 | `stdlib/net/websocket.ny` |
| `ws_send_text_server` | `int ws_send_text_server(int handle, const char *text)` | `rt_websocket.c` | 1.11.0 | `stdlib/net/websocket.ny` |
| `x509_available` | `int x509_available(void)` | `rt_crypto_openssl.c` | 1.3.3 | `stdlib/crypto/x509.ny` |
| `x509_pem_issuer` | `char *x509_pem_issuer(const char *pem_cert)` | `rt_crypto_openssl.c` | 1.3.3 | `stdlib/crypto/x509.ny` |
| `x509_pem_subject` | `char *x509_pem_subject(const char *pem_cert)` | `rt_crypto_openssl.c` | 1.3.3 | `stdlib/crypto/x509.ny` |
| `x509_pem_verify_time` | `int x509_pem_verify_time(const char *pem_cert)` | `rt_crypto_openssl.c` | 1.3.3 | `stdlib/crypto/x509.ny` |
| `zip_create_file` | `int zip_create_file(const char *archive_path, const char *source_path, const char *entry_name)` | `rt_zip.c` | 1.3.3 | `stdlib/archive/zip.ny` |
| `zip_extract_file` | `int zip_extract_file(const char *archive_path, const char *dest_path)` | `rt_zip.c` | 1.3.3 | `stdlib/archive/zip.ny` |

## Experimental bindings

| Symbol | C signature | RT module | Since | Nyra stdlib |
|--------|-------------|-----------|-------|-------------|
| `_mysql_stub_open` | `void *_mysql_stub_open(const char *dsn)` | `rt_db.c` | 1.3.3 | `stdlib/db/mysql.ny` |
| `_postgres_stub_open` | `void *_postgres_stub_open(const char *dsn)` | `rt_db.c` | 1.3.3 | `stdlib/db/postgres.ny` |
| `_sqlite_null_handle` | `void *_sqlite_null_handle(void)` | `rt_db.c` | 1.3.1 | `stdlib/db/sql.ny` |
| `asm_nop` | `void asm_nop(void)` | `rt_syscall.c` | 0.5.0 | `stdlib/os/syscall.ny` |
| `asm_pause` | `void asm_pause(void)` | `rt_syscall.c` | 0.5.0 | `stdlib/os/syscall.ny` |
| `blackbox_i32` | `void blackbox_i32(void)` | `rt_bench.c` | 0.5.0 | `stdlib/bench/mod.ny` |
| `bounds_assert_i32` | `void bounds_assert_i32(int ok)` | `rt_array.c` | 1.3.0 | — |
| `hw_cpu_brand` | `char *hw_cpu_brand(void)` | `rt_hw.c` | 1.3.0 | `stdlib/os/cpu.ny` |
| `hw_cpu_cache_line_size` | `int32_t hw_cpu_cache_line_size(void)` | `rt_hw.c` | 1.3.0 | `stdlib/os/cpu.ny` |
| `hw_cpu_has_avx` | `int32_t hw_cpu_has_avx(void)` | `rt_hw.c` | 1.3.0 | `stdlib/os/cpu.ny` |
| `hw_cpu_has_avx2` | `int32_t hw_cpu_has_avx2(void)` | `rt_hw.c` | 1.3.0 | `stdlib/os/cpu.ny` |
| `hw_cpu_has_sse42` | `int32_t hw_cpu_has_sse42(void)` | `rt_hw.c` | 1.3.0 | `stdlib/os/cpu.ny` |
| `hw_cpu_logical_cores` | `int32_t hw_cpu_logical_cores(void)` | `rt_hw.c` | 1.3.0 | `stdlib/os/cpu.ny` |
| `hw_cpu_physical_cores` | `int32_t hw_cpu_physical_cores(void)` | `rt_hw.c` | 1.3.0 | `stdlib/os/cpu.ny` |
| `hw_disk_free_bytes` | `int64_t hw_disk_free_bytes(const char *path)` | `rt_hw.c` | 1.3.0 | `stdlib/os/storage.ny` |
| `hw_disk_fs_type` | `char *hw_disk_fs_type(const char *path)` | `rt_hw.c` | 1.3.0 | `stdlib/os/storage.ny` |
| `hw_disk_total_bytes` | `int64_t hw_disk_total_bytes(const char *path)` | `rt_hw.c` | 1.3.0 | `stdlib/os/storage.ny` |
| `hw_display_brightness_pct` | `int32_t hw_display_brightness_pct(void)` | `rt_hw.c` | 1.3.0 | `stdlib/os/display.ny` |
| `hw_display_height` | `int32_t hw_display_height(void)` | `rt_hw.c` | 1.3.0 | `stdlib/os/display.ny` |
| `hw_display_refresh_hz` | `int32_t hw_display_refresh_hz(void)` | `rt_hw.c` | 1.3.0 | `stdlib/os/display.ny` |
| `hw_display_width` | `int32_t hw_display_width(void)` | `rt_hw.c` | 1.3.0 | `stdlib/os/display.ny` |
| `hw_dma_available` | `int32_t hw_dma_available(void)` | `rt_hw.c` | 1.3.0 | `stdlib/os/memory.ny` |
| `hw_mem_map_anonymous` | `void *hw_mem_map_anonymous(int64_t nbytes)` | `rt_hw.c` | 1.3.0 | `stdlib/os/memory.ny` |
| `hw_mem_map_file` | `void *hw_mem_map_file(const char *path, int64_t nbytes, int32_t writable)` | `rt_hw.c` | 1.39.0 | `stdlib/os/memory.ny` |
| `hw_mem_page_size` | `int32_t hw_mem_page_size(void)` | `rt_hw.c` | 1.3.0 | `stdlib/os/memory.ny` |
| `hw_mem_sync` | `int32_t hw_mem_sync(void *addr, int64_t nbytes)` | `rt_hw.c` | 1.39.0 | `stdlib/os/memory.ny` |
| `hw_mem_unmap` | `int32_t hw_mem_unmap(void *addr, int64_t nbytes)` | `rt_hw.c` | 1.3.0 | `stdlib/os/memory.ny` |
| `hw_net_if_count` | `int32_t hw_net_if_count(void)` | `rt_hw.c` | 1.3.0 | `stdlib/os/netif.ny` |
| `hw_net_if_is_up` | `int32_t hw_net_if_is_up(int32_t index)` | `rt_hw.c` | 1.3.0 | `stdlib/os/netif.ny` |
| `hw_net_if_mac` | `char *hw_net_if_mac(int32_t index)` | `rt_hw.c` | 1.3.0 | `stdlib/os/netif.ny` |
| `hw_net_if_name` | `char *hw_net_if_name(int32_t index)` | `rt_hw.c` | 1.3.0 | `stdlib/os/netif.ny` |
| `hw_power_cpu_temp_centi_c` | `int32_t hw_power_cpu_temp_centi_c(void)` | `rt_hw.c` | 1.3.0 | `stdlib/os/power.ny` |
| `hw_power_on_ac` | `int32_t hw_power_on_ac(void)` | `rt_hw.c` | 1.3.0 | `stdlib/os/power.ny` |
| `mysql_exec` | `int mysql_exec(void *handle, const char *sql)` | `rt_db.c` | 1.3.3 | `stdlib/db/mysql.ny` |
| `nyra_mysql_close` | `void nyra_mysql_close(void *handle)` | `rt_db.c` | 1.3.3 | `stdlib/db/mysql.ny` |
| `os_battery_percent` | `void os_battery_percent(void)` | `rt_os.c` | 0.5.0 | `stdlib/os/battery.ny` |
| `os_close_fd` | `void os_close_fd(void)` | `rt_syscall.c` | 0.5.0 | `stdlib/os/syscall.ny` |
| `os_exit` | `void os_exit(void)` | `rt_syscall.c` | 0.5.0 | `stdlib/os/syscall.ny` |
| `os_getpid` | `void os_getpid(void)` | `rt_syscall.c` | 0.5.0 | `stdlib/os/syscall.ny` |
| `os_page_size` | `void os_page_size(void)` | `rt_os.c` | 0.5.0 | `stdlib/os/platform.ny` |
| `os_platform_id` | `void os_platform_id(void)` | `rt_os.c` | 0.5.0 | `stdlib/os/platform.ny` |
| `os_platform_name` | `void os_platform_name(void)` | `rt_os.c` | 0.5.0 | `stdlib/os/platform.ny` |
| `os_read` | `void os_read(void)` | `rt_syscall.c` | 0.5.0 | `stdlib/os/syscall.ny` |
| `os_syscall6` | `void os_syscall6(void)` | `rt_syscall.c` | 0.5.0 | `stdlib/os/syscall.ny` |
| `os_write` | `void os_write(void)` | `rt_syscall.c` | 0.5.0 | `stdlib/os/syscall.ny` |
| `postgres_close` | `void postgres_close(void *handle)` | `rt_db.c` | 1.3.3 | `stdlib/db/postgres.ny` |
| `postgres_exec` | `int postgres_exec(void *handle, const char *sql)` | `rt_db.c` | 1.3.3 | `stdlib/db/postgres.ny` |
| `race_clear_access` | `void race_clear_access(void *addr)` | `rt_race.c` | 1.9.0 | `stdlib/race.ny` |
| `race_runtime_enabled` | `int race_runtime_enabled(void)` | `rt_race.c` | 1.9.0 | `stdlib/race.ny` |
| `race_runtime_init` | `void race_runtime_init(void)` | `rt_race.c` | 1.9.0 | `stdlib/race.ny` |
| `race_track_read` | `void race_track_read(void *addr, int64_t nbytes)` | `rt_race.c` | 1.9.0 | `stdlib/race.ny` |
| `race_track_write` | `void race_track_write(void *addr, int64_t nbytes)` | `rt_race.c` | 1.9.0 | `stdlib/race.ny` |
| `rt_affinity_get_thread_cpu` | `int32_t rt_affinity_get_thread_cpu(void)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/affinity.ny` |
| `rt_affinity_set_thread_cpu` | `int32_t rt_affinity_set_thread_cpu(int32_t core_index)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/affinity.ny` |
| `rt_clock_monotonic_ns` | `int64_t rt_clock_monotonic_ns(void)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/clocks.ny` |
| `rt_clock_rdtsc` | `int64_t rt_clock_rdtsc(void)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/clocks.ny` |
| `rt_hw_random_bytes` | `char *rt_hw_random_bytes(int32_t count)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/hw_crypto.ny` |
| `rt_hw_secure_enclave_available` | `int32_t rt_hw_secure_enclave_available(void)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/hw_crypto.ny` |
| `rt_mqueue_close` | `int32_t rt_mqueue_close(int32_t mq_id)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/mqueue.ny` |
| `rt_mqueue_open` | `int32_t rt_mqueue_open(const char *name, int32_t max_msgs, int32_t msg_size)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/mqueue.ny` |
| `rt_mqueue_recv` | `char *rt_mqueue_recv(int32_t mq_id, int32_t max_bytes)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/mqueue.ny` |
| `rt_mqueue_send` | `int32_t rt_mqueue_send(int32_t mq_id, const char *msg)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/mqueue.ny` |
| `rt_os_getenv` | `void rt_os_getenv(void)` | `rt_os.c` | 0.5.0 | `stdlib/os/env.ny` |
| `rt_os_setenv` | `int rt_os_setenv(const char *name, const char *value)` | `rt_os.c` | 0.5.0 | `stdlib/env/mod.ny` |
| `rt_perm_chroot` | `int32_t rt_perm_chroot(const char *path)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/permissions.ny` |
| `rt_perm_drop_to_uid` | `int32_t rt_perm_drop_to_uid(int32_t uid)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/permissions.ny` |
| `rt_perm_geteuid` | `int32_t rt_perm_geteuid(void)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/permissions.ny` |
| `rt_perm_getuid` | `int32_t rt_perm_getuid(void)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/permissions.ny` |
| `rt_perm_sandbox_seatbelt_available` | `int32_t rt_perm_sandbox_seatbelt_available(void)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/permissions.ny` |
| `rt_serial_close` | `int32_t rt_serial_close(int32_t handle)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/serial.ny` |
| `rt_serial_open` | `int32_t rt_serial_open(const char *path, int32_t baud)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/serial.ny` |
| `rt_serial_read` | `char *rt_serial_read(int32_t handle, int32_t max_bytes)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/serial.ny` |
| `rt_serial_write` | `int32_t rt_serial_write(int32_t handle, const char *data)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/serial.ny` |
| `rt_signal_install` | `int32_t rt_signal_install(int32_t sig_num)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/signals.ny` |
| `rt_signal_poll` | `int32_t rt_signal_poll(int32_t sig_num)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/signals.ny` |
| `rt_usb_device_count` | `int32_t rt_usb_device_count(void)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/usb.ny` |
| `rt_usb_device_path` | `char *rt_usb_device_path(int32_t index)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/usb.ny` |
| `rt_usb_device_pid` | `int32_t rt_usb_device_pid(int32_t index)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/usb.ny` |
| `rt_usb_device_vid` | `int32_t rt_usb_device_vid(int32_t index)` | `rt_os_adv.c` | 1.4.0 | `stdlib/os/usb.ny` |
| `volatile_load_i32` | `void volatile_load_i32(void)` | `rt_volatile.c` | 0.5.0 | `stdlib/core/mem.ny` |
| `volatile_load_u32` | `void volatile_load_u32(void)` | `rt_volatile.c` | 0.5.0 | `stdlib/core/mem.ny` |
| `volatile_store_i32` | `void volatile_store_i32(void)` | `rt_volatile.c` | 0.5.0 | `stdlib/core/mem.ny` |
| `volatile_store_u32` | `void volatile_store_u32(void)` | `rt_volatile.c` | 0.5.0 | `stdlib/core/mem.ny` |

## Package bindings (NyraPkg)

Third-party packages ship their own `extern fn` + `link-source` C shims. Example:

| Package | Nyra module | C shim | Native lib |
|---------|-------------|--------|------------|
| `ny-sqlite` | `examples/packages/ny-sqlite/sqlite.ny` | `rt/sqlite.c` | `-lsqlite3` |

Install with `nyra pkg install ny-sqlite@^0.1.0` then `import "pkg/ny-sqlite"`.

See [`docs/nyrapkg-v1.md`](nyrapkg-v1.md) and [`docs/integration-ideas/native-bindings/README.md`](integration-ideas/native-bindings/README.md).
