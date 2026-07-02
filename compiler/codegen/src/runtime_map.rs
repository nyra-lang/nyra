use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

/// Symbols referenced from generated LLVM IR → runtime source file under `stdlib/rt/`.
pub fn symbol_module_map() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("heap_free", "rt_alloc.c"),
        ("str_clone", "rt_alloc.c"),
        ("str_len", "rt_strings.c"),
        ("str_cat", "rt_strings.c"),
        ("i32_to_string", "rt_strings.c"),
        ("i64_to_string", "rt_strings.c"),
        ("str_cmp", "rt_strings.c"),
        ("char_at", "rt_strings.c"),
        ("substring", "rt_strings.c"),
        ("strstr_pos", "rt_strings.c"),
        ("str_to_upper", "rt_strings.c"),
        ("str_to_lower", "rt_strings.c"),
        ("str_trim", "rt_strings.c"),
        ("str_contains", "rt_strings.c"),
        ("str_starts_with", "rt_strings.c"),
        ("str_ends_with", "rt_strings.c"),
        ("str_replace", "rt_strings.c"),
        ("str_replacen", "rt_strings.c"),
        ("str_split", "rt_strings.c"),
        ("str_to_i32", "rt_strings.c"),
        ("str_to_f64", "rt_strings.c"),
        ("f64_to_string", "rt_strings.c"),
        ("str_push_char", "rt_strings.c"),
        ("str_pop", "rt_strings.c"),
        ("str_strip_ansi", "rt_strings.c"),
        ("str_buf_new", "rt_str_buf.c"),
        ("str_buf_drop", "rt_str_buf.c"),
        ("str_buf_append", "rt_str_buf.c"),
        ("str_buf_append_char", "rt_str_buf.c"),
        ("str_buf_build", "rt_str_buf.c"),
        ("pty_spawn", "rt_pty.c"),
        ("pty_write", "rt_pty.c"),
        ("pty_read", "rt_pty.c"),
        ("pty_drain", "rt_pty.c"),
        ("pty_drain_raw", "rt_pty.c"),
        ("pty_read_wait_raw", "rt_pty.c"),
        ("pty_flush_stdout", "rt_pty.c"),
        ("pty_read_wait", "rt_pty.c"),
        ("pty_poll", "rt_pty.c"),
        ("pty_resize", "rt_pty.c"),
        ("pty_close", "rt_pty.c"),
        ("pty_wait", "rt_pty.c"),
        ("gpu_font_init", "gpu/rt_gpu_font.c"),
        ("gpu_font_draw", "gpu/rt_gpu_font.c"),
        ("gpu_font_free", "gpu/rt_gpu_font.c"),
        ("array_i32_sort_copy", "rt_array.c"),
        ("bounds_assert_i32", "rt_array.c"),
        ("array_f64_sort_copy", "rt_array.c"),
        ("array_i32_debug_string", "rt_array.c"),
        ("array_f64_debug_string", "rt_array.c"),
        ("array_f32_debug_string", "rt_array.c"),
        ("array_bool_debug_string", "rt_array.c"),
        ("array_str_debug_string", "rt_array.c"),
        ("vec_str_new", "rt_vec.c"),
        ("vec_str_push", "rt_vec.c"),
        ("vec_str_get", "rt_vec.c"),
        ("vec_str_len", "rt_vec.c"),
        ("vec_str_free", "rt_vec.c"),
        ("read_file", "rt_fs.c"),
        ("read_file_limit", "rt_fs.c"),
        ("write_file", "rt_fs.c"),
        ("file_exists", "rt_fs.c"),
        ("append_file", "rt_fs.c"),
        ("fsync_file", "rt_fs.c"),
        ("nyra_check_file", "rt_compiler.c"),
        ("nyra_check_source", "rt_compiler.c"),
        ("nyra_diag_json_file", "rt_compiler.c"),
        ("nyra_diag_json_source", "rt_compiler.c"),
        ("nyra_compiler_free", "rt_compiler.c"),
        ("remove_file", "rt_fs.c"),
        ("create_dir", "rt_fs.c"),
        ("create_dir_all", "rt_fs.c"),
        ("remove_dir", "rt_fs.c"),
        ("remove_dir_all", "rt_fs.c"),
        ("file_size", "rt_fs.c"),
        ("copy_file", "rt_fs.c"),
        ("copy_dir", "rt_fs.c"),
        ("copy_dir_contents", "rt_fs.c"),
        ("list_dir", "rt_fs.c"),
        ("path_is_dir", "rt_fs.c"),
        ("rt_args_init", "rt_args.c"),
        ("os_arg_count", "rt_args.c"),
        ("os_arg_at", "rt_args.c"),
        ("process_exit", "rt_args.c"),
        ("vec_str_from_argv", "rt_args.c"),
        ("bytes_read_file", "rt_bytes.c"),
        ("bytes_len", "rt_bytes.c"),
        ("byte_at", "rt_bytes.c"),
        ("bytes_write_file", "rt_bytes.c"),
        ("bytes_from_string", "rt_bytes.c"),
        ("bytes_to_string", "rt_bytes.c"),
        ("bytes_free", "rt_bytes.c"),
        ("stdin_read_bytes", "rt_bytes.c"),
        ("stdout_write_bytes", "rt_bytes.c"),
        ("arena_new", "rt_arena.c"),
        ("arena_alloc", "rt_arena.c"),
        ("arena_reset", "rt_arena.c"),
        ("arena_free", "rt_arena.c"),
        ("regex_compile", "rt_regex.c"),
        ("regex_is_match", "rt_regex.c"),
        ("regex_free", "rt_regex.c"),
        ("tar_create", "rt_tar.c"),
        ("tar_extract", "rt_tar.c"),
        ("gzip_file", "rt_gzip.c"),
        ("gunzip_file", "rt_gzip.c"),
        ("gzip_compress_hex", "rt_gzip.c"),
        ("gzip_decompress_hex", "rt_gzip.c"),
        ("flate_compress_hex", "rt_gzip.c"),
        ("flate_decompress_hex", "rt_gzip.c"),
        ("ws_connect", "rt_websocket.c"),
        ("ws_send_text", "rt_websocket.c"),
        ("ws_recv_text", "rt_websocket.c"),
        ("ws_close", "rt_websocket.c"),
        ("ws_listen", "rt_websocket.c"),
        ("ws_listen_tls", "rt_websocket.c"),
        ("ws_accept_handshake", "rt_websocket.c"),
        ("ws_accept_tls_handshake", "rt_websocket.c"),
        ("ws_send_text_server", "rt_websocket.c"),
        ("stdout_write_str", "rt_io.c"),
        ("stdout_writeln_str", "rt_io.c"),
        ("stdout_write_i32", "rt_io.c"),
        ("stdout_writeln_i32", "rt_io.c"),
        ("stdout_flush", "rt_io.c"),
        ("stdin_read_line", "rt_io.c"),
        ("println", "rt_io.c"),
        ("color_ansi", "rt_io.c"),
        ("ansi_reset", "rt_io.c"),
        ("time_start", "rt_time.c"),
        ("time_end", "rt_time.c"),
        ("instant_now", "rt_time.c"),
        ("instant_elapsed_ms", "rt_time.c"),
        ("sleep_ms", "rt_time.c"),
        ("date_now", "rt_time.c"),
        ("mem_start", "rt_mem.c"),
        ("mem_end", "rt_mem.c"),
        ("_sqlite_null_handle", "rt_db.c"),
        ("spawn_capture", "rt_spawn.c"),
        ("spawn_join", "rt_spawn.c"),
        ("spawn_handle_drop", "rt_spawn.c"),
        ("spawn_task_capture", "rt_task_pool.c"),
        ("spawn_task_join", "rt_task_pool.c"),
        ("spawn_task_handle_drop", "rt_task_pool.c"),
        ("parallel_for_range", "rt_parallel.c"),
        ("parallel_any_range", "rt_parallel.c"),
        ("parallel_find_range", "rt_parallel.c"),
        ("parallel_all_range", "rt_parallel.c"),
        ("cpu_count", "rt_parallel.c"),
        ("progress_update", "rt_progress.c"),
        ("progress_finish", "rt_progress.c"),
        ("benchmark_begin", "rt_bench.c"),
        ("benchmark_end", "rt_bench.c"),
        ("spawn", "rt_async.c"),
        ("async_await", "rt_async.c"),
        ("async_run", "rt_async.c"),
        ("async_promise_new", "rt_async.c"),
        ("async_promise_complete", "rt_async.c"),
        ("async_promise_complete_bool", "rt_async.c"),
        ("async_promise_complete_ptr", "rt_async.c"),
        ("async_poll", "rt_async.c"),
        ("async_poll_bool", "rt_async.c"),
        ("async_await_bool", "rt_async.c"),
        ("async_await_ptr", "rt_async.c"),
        ("async_future_done", "rt_async.c"),
        ("async_future_ptr_value", "rt_async.c"),
        ("async_select2_i32", "rt_async.c"),
        ("async_select2_bool", "rt_async.c"),
        ("async_select2_ptr", "rt_async.c"),
        ("async_select_i32", "rt_async.c"),
        ("runtime_run", "rt_async.c"),
        ("runtime_poll_io", "rt_async.c"),
        ("runtime_executor_tick", "rt_async.c"),
        ("runtime_executor_run_until", "rt_async.c"),
        ("async_sleep_ms", "rt_async.c"),
        ("io_register", "rt_async.c"),
        ("io_unregister", "rt_async.c"),
        ("io_wait_once", "rt_async.c"),
        ("rt_tcp_listen", "rt_net.c"),
        ("rt_tcp_accept", "rt_net.c"),
        ("rt_tcp_accept_async", "rt_net.c"),
        ("rt_tcp_connect", "rt_net.c"),
        ("rt_tcp_connect_timeout", "rt_net.c"),
        ("rt_dns_lookup", "rt_net.c"),
        ("rt_tcp_ping_ms", "rt_net.c"),
        ("rt_icmp_ping_ms", "rt_net.c"),
        ("rt_icmp_ping_system_ms", "rt_net.c"),
        ("rt_icmp_capable", "rt_net.c"),
        ("rt_tcp_read", "rt_net.c"),
        ("rt_tcp_write", "rt_net.c"),
        ("rt_tcp_close", "rt_net.c"),
        ("sys_listen", "rt_net.c"),
        ("sys_accept", "rt_net.c"),
        ("sys_connect", "rt_net.c"),
        ("sys_recv", "rt_net.c"),
        ("sys_send", "rt_net.c"),
        ("sys_close", "rt_net.c"),
        ("sys_set_nonblock", "rt_net.c"),
        ("rt_udp_bind", "rt_net.c"),
        ("rt_udp_recv", "rt_net.c"),
        ("rt_udp_send", "rt_net.c"),
        ("rt_udp_close", "rt_net.c"),
        ("zip_create_file", "rt_zip.c"),
        ("zip_extract_file", "rt_zip.c"),
        ("_postgres_stub_open", "rt_db.c"),
        ("postgres_exec", "rt_db.c"),
        ("postgres_close", "rt_db.c"),
        ("_mysql_stub_open", "rt_db.c"),
        ("mysql_exec", "rt_db.c"),
        ("mysql_close", "rt_db.c"),
        ("http_get", "rt_http.c"),
        ("http_download_file", "rt_http.c"),
        ("http_status", "rt_http.c"),
        ("json_get_string", "rt_json.c"),
        ("json_get_i32", "rt_json.c"),
        ("json_get_bool", "rt_json.c"),
        ("json_get_object", "rt_json.c"),
        ("json_get_array", "rt_json.c"),
        ("json_encode_object", "rt_json.c"),
        ("json_encode_i32_array", "rt_json.c"),
        ("json_decode_i32_array", "rt_json.c"),
        ("json_encode_str_array", "rt_json.c"),
        ("json_join_raw_array", "rt_json.c"),
        ("json_decode_str_array", "rt_json.c"),
        ("json_split_array_elements", "rt_json.c"),
        ("json_encode_ptr_token", "rt_json.c"),
        ("json_decode_ptr_token", "rt_json.c"),
        ("bin_buf_new", "rt_bin.c"),
        ("bin_buf_write_i32", "rt_bin.c"),
        ("bin_buf_write_bool", "rt_bin.c"),
        ("bin_buf_write_string", "rt_bin.c"),
        ("bin_buf_write_bytes", "rt_bin.c"),
        ("bin_buf_append_blob", "rt_bin.c"),
        ("bin_buf_finish", "rt_bin.c"),
        ("bin_blob_payload_len", "rt_bin.c"),
        ("bin_decode_i32_at", "rt_bin.c"),
        ("bin_decode_bool_at", "rt_bin.c"),
        ("bin_decode_string_at", "rt_bin.c"),
        ("bin_decode_blob_at", "rt_bin.c"),
        ("bin_field_width_string_at", "rt_bin.c"),
        ("bin_field_width_blob_at", "rt_bin.c"),
        ("bin_field_width_i32", "rt_bin.c"),
        ("bin_field_width_bool", "rt_bin.c"),
        ("bin_blob_free", "rt_bin.c"),
        ("race_runtime_init", "rt_race.c"),
        ("race_track_read", "rt_race.c"),
        ("race_track_write", "rt_race.c"),
        ("race_clear_access", "rt_race.c"),
        ("race_runtime_enabled", "rt_race.c"),
        ("sha256_hex", "rt_crypto.c"),
        ("sha512_hex", "rt_crypto_sha512.c"),
        ("hmac_sha256_hex", "rt_crypto.c"),
        ("aes_cbc_encrypt_hex", "rt_aes.c"),
        ("aes_cbc_decrypt_hex", "rt_aes.c"),
        ("sqlite_open", "rt_sqlite.c"),
        ("sqlite_exec", "rt_sqlite.c"),
        ("sqlite_close", "rt_sqlite.c"),
        ("sqlite_last_error", "rt_sqlite.c"),
        ("sqlite_prepare", "rt_sqlite.c"),
        ("sqlite_step", "rt_sqlite.c"),
        ("sqlite_column_count", "rt_sqlite.c"),
        ("sqlite_column_text", "rt_sqlite.c"),
        ("sqlite_finalize", "rt_sqlite.c"),
        ("sqlite_query_rows", "rt_sqlite.c"),
        ("sqlite_rowset_rows", "rt_sqlite.c"),
        ("sqlite_rowset_cols", "rt_sqlite.c"),
        ("sqlite_rowset_at", "rt_sqlite.c"),
        ("sqlite_rowset_free", "rt_sqlite.c"),
        ("tls_available", "rt_tls.c"),
        ("rt_tls_connect", "rt_tls.c"),
        ("rt_tls_connect_verify", "rt_tls.c"),
        ("rt_tls_connect_ca", "rt_tls.c"),
        ("rt_tls_connect_ex", "rt_tls.c"),
        ("rt_tls_upgrade_client", "rt_tls.c"),
        ("rt_tls_upgrade_client_verify", "rt_tls.c"),
        ("rt_tls_upgrade_client_ex", "rt_tls.c"),
        ("rt_tls_last_error", "rt_tls.c"),
        ("rt_tls_validate_pem_files", "rt_tls.c"),
        ("rt_tls_read", "rt_tls.c"),
        ("rt_tls_write", "rt_tls.c"),
        ("rt_tls_close", "rt_tls.c"),
        ("rt_tls_gen_self_signed", "rt_tls.c"),
        ("rt_tls_listen", "rt_tls.c"),
        ("rt_tls_accept", "rt_tls.c"),
        ("rt_tls_listener_close", "rt_tls.c"),
        ("rsa_available", "rt_crypto_openssl.c"),
        ("rsa_public_encrypt_pem", "rt_crypto_openssl.c"),
        ("rsa_sha256_sign_pem", "rt_crypto_openssl.c"),
        ("x509_available", "rt_crypto_openssl.c"),
        ("x509_pem_subject", "rt_crypto_openssl.c"),
        ("x509_pem_issuer", "rt_crypto_openssl.c"),
        ("x509_pem_verify_time", "rt_crypto_openssl.c"),
        ("channel_new", "rt_channel.c"),
        ("channel_send", "rt_channel.c"),
        ("channel_recv", "rt_channel.c"),
        ("channel_free", "rt_channel.c"),
        ("channel_str_new", "rt_channel.c"),
        ("channel_str_send", "rt_channel.c"),
        ("channel_str_recv", "rt_channel.c"),
        ("channel_str_free", "rt_channel.c"),
        ("rt_tcp_hub_new", "rt_tcp_hub.c"),
        ("rt_tcp_hub_add", "rt_tcp_hub.c"),
        ("rt_tcp_hub_remove", "rt_tcp_hub.c"),
        ("rt_tcp_hub_broadcast", "rt_tcp_hub.c"),
        ("rt_tcp_hub_free", "rt_tcp_hub.c"),
        ("mutex_new", "rt_sync.c"),
        ("mutex_lock", "rt_sync.c"),
        ("mutex_unlock", "rt_sync.c"),
        ("mutex_free", "rt_sync.c"),
        ("rwlock_new", "rt_sync.c"),
        ("rwlock_rlock", "rt_sync.c"),
        ("rwlock_wlock", "rt_sync.c"),
        ("rwlock_unlock", "rt_sync.c"),
        ("rwlock_free", "rt_sync.c"),
        ("waitgroup_new", "rt_sync.c"),
        ("waitgroup_add", "rt_sync.c"),
        ("waitgroup_done", "rt_sync.c"),
        ("waitgroup_wait", "rt_sync.c"),
        ("waitgroup_free", "rt_sync.c"),
        ("atomic_load_i32", "rt_sync.c"),
        ("atomic_store_i32", "rt_sync.c"),
        ("atomic_add_i32", "rt_sync.c"),
        ("atomic_cas_i32", "rt_sync.c"),
        ("atomic_i32_new", "rt_sync.c"),
        ("atomic_i32_free", "rt_sync.c"),
        ("vec_i32_new", "rt_vec.c"),
        ("vec_i32_push", "rt_vec.c"),
        ("vec_i32_get", "rt_vec.c"),
        ("vec_i32_set", "rt_vec.c"),
        ("vec_i32_len", "rt_vec.c"),
        ("vec_i32_pop", "rt_vec.c"),
        ("vec_i32_free", "rt_vec.c"),
        ("vec_bytes_new", "rt_vec.c"),
        ("vec_bytes_push", "rt_vec.c"),
        ("vec_bytes_get", "rt_vec.c"),
        ("vec_bytes_len", "rt_vec.c"),
        ("vec_bytes_free", "rt_vec.c"),
        ("vec_bytes_push_ptr", "rt_vec.c"),
        ("vec_bytes_get_ptr", "rt_vec.c"),
        ("map_str_i32_new", "rt_map.c"),
        ("map_str_i32_insert", "rt_map.c"),
        ("map_str_i32_get", "rt_map.c"),
        ("map_str_i32_contains", "rt_map.c"),
        ("map_str_i32_keys", "rt_map.c"),
        ("map_str_i32_remove", "rt_map.c"),
        ("map_str_i32_free", "rt_map.c"),
        ("map_str_i32_retain", "rt_map.c"),
        ("map_i32_i32_new", "rt_map.c"),
        ("map_i32_i32_insert", "rt_map.c"),
        ("map_i32_i32_get", "rt_map.c"),
        ("map_i32_i32_contains", "rt_map.c"),
        ("map_i32_i32_free", "rt_map.c"),
        ("map_i32_i32_retain", "rt_map.c"),
        ("map_str_str_new", "rt_map_str_str.c"),
        ("map_str_str_insert", "rt_map_str_str.c"),
        ("map_str_str_get", "rt_map_str_str.c"),
        ("map_str_str_contains", "rt_map_str_str.c"),
        ("map_str_str_keys", "rt_map_str_str.c"),
        ("map_str_str_remove", "rt_map_str_str.c"),
        ("map_str_str_free", "rt_map_str_str.c"),
        ("map_str_str_retain", "rt_map_str_str.c"),
        ("rand_i32", "rt_random.c"),
        ("rand_range", "rt_random.c"),
        ("rand_i64", "rt_random.c"),
        ("rand_range_i64", "rt_random.c"),
        ("rand_u32", "rt_random.c"),
        ("rand_range_u32", "rt_random.c"),
        ("rand_u64", "rt_random.c"),
        ("rand_range_u64", "rt_random.c"),
        ("random_hex", "rt_random.c"),
        ("rand_f64", "rt_random.c"),
        ("rand_f64_range", "rt_random.c"),
        ("sin_f64", "rt_math.c"),
        ("cos_f64", "rt_math.c"),
        ("atan2_f64", "rt_math.c"),
        ("tan_f64", "rt_math.c"),
        ("stdin_set_raw_mode", "rt_io.c"),
        ("stdin_read_key", "rt_io.c"),
        ("blackbox_i32", "rt_bench.c"),
        ("volatile_load_i32", "rt_volatile.c"),
        ("volatile_store_i32", "rt_volatile.c"),
        ("volatile_load_u32", "rt_volatile.c"),
        ("volatile_store_u32", "rt_volatile.c"),
        ("os_syscall6", "rt_syscall.c"),
        ("os_getpid", "rt_syscall.c"),
        ("os_exit", "rt_syscall.c"),
        ("os_read", "rt_syscall.c"),
        ("os_write", "rt_syscall.c"),
        ("os_close_fd", "rt_syscall.c"),
        ("asm_nop", "rt_syscall.c"),
        ("asm_pause", "rt_syscall.c"),
        ("os_platform_id", "rt_os.c"),
        ("os_platform_name", "rt_os.c"),
        ("rt_os_getenv", "rt_os.c"),
        ("rt_os_setenv", "rt_os.c"),
        ("os_battery_percent", "rt_os.c"),
        ("os_page_size", "rt_os.c"),
        ("hw_cpu_physical_cores", "rt_hw.c"),
        ("hw_cpu_logical_cores", "rt_hw.c"),
        ("hw_cpu_cache_line_size", "rt_hw.c"),
        ("hw_cpu_has_sse42", "rt_hw.c"),
        ("hw_cpu_has_avx", "rt_hw.c"),
        ("hw_cpu_has_avx2", "rt_hw.c"),
        ("hw_cpu_brand", "rt_hw.c"),
        ("hw_mem_page_size", "rt_hw.c"),
        ("hw_mem_map_anonymous", "rt_hw.c"),
        ("hw_mem_map_file", "rt_hw.c"),
        ("hw_mem_sync", "rt_hw.c"),
        ("hw_mem_unmap", "rt_hw.c"),
        ("hw_dma_available", "rt_hw.c"),
        ("shm_create", "rt_shm.c"),
        ("shm_open_existing", "rt_shm.c"),
        ("shm_map", "rt_shm.c"),
        ("shm_unmap", "rt_shm.c"),
        ("shm_close_fd", "rt_shm.c"),
        ("shm_unlink_region", "rt_shm.c"),
        ("io_pool_create", "rt_io_pool.c"),
        ("io_pool_shutdown", "rt_io_pool.c"),
        ("io_pool_submit_wait_readable", "rt_io_pool.c"),
        ("io_pool_submit_read", "rt_io_pool.c"),
        ("io_pool_queue_depth", "rt_io_pool.c"),
        ("io_uring_available", "rt_io_uring.c"),
        ("io_uring_register_read", "rt_io_uring.c"),
        ("io_uring_unregister_read", "rt_io_uring.c"),
        ("io_uring_pending", "rt_io_uring.c"),
        ("io_uring_wait_once", "rt_io_uring.c"),
        ("hw_disk_total_bytes", "rt_hw.c"),
        ("hw_disk_free_bytes", "rt_hw.c"),
        ("hw_disk_fs_type", "rt_hw.c"),
        ("hw_net_if_count", "rt_hw.c"),
        ("hw_net_if_name", "rt_hw.c"),
        ("hw_net_if_mac", "rt_hw.c"),
        ("hw_net_if_is_up", "rt_hw.c"),
        ("hw_display_width", "rt_hw.c"),
        ("hw_display_height", "rt_hw.c"),
        ("hw_display_refresh_hz", "rt_hw.c"),
        ("hw_display_brightness_pct", "rt_hw.c"),
        ("hw_power_on_ac", "rt_hw.c"),
        ("hw_power_cpu_temp_centi_c", "rt_hw.c"),
        ("rt_affinity_set_thread_cpu", "rt_os_adv.c"),
        ("rt_affinity_get_thread_cpu", "rt_os_adv.c"),
        ("rt_clock_rdtsc", "rt_os_adv.c"),
        ("rt_clock_monotonic_ns", "rt_os_adv.c"),
        ("rt_usb_device_count", "rt_os_adv.c"),
        ("rt_usb_device_vid", "rt_os_adv.c"),
        ("rt_usb_device_pid", "rt_os_adv.c"),
        ("rt_usb_device_path", "rt_os_adv.c"),
        ("rt_serial_open", "rt_os_adv.c"),
        ("rt_serial_read", "rt_os_adv.c"),
        ("rt_serial_write", "rt_os_adv.c"),
        ("rt_serial_close", "rt_os_adv.c"),
        ("rt_signal_install", "rt_os_adv.c"),
        ("rt_signal_poll", "rt_os_adv.c"),
        ("rt_mqueue_open", "rt_os_adv.c"),
        ("rt_mqueue_send", "rt_os_adv.c"),
        ("rt_mqueue_recv", "rt_os_adv.c"),
        ("rt_mqueue_close", "rt_os_adv.c"),
        ("rt_hw_random_bytes", "rt_os_adv.c"),
        ("rt_hw_secure_enclave_available", "rt_os_adv.c"),
        ("rt_perm_getuid", "rt_os_adv.c"),
        ("rt_perm_geteuid", "rt_os_adv.c"),
        ("rt_perm_drop_to_uid", "rt_os_adv.c"),
        ("rt_perm_chroot", "rt_os_adv.c"),
        ("rt_perm_sandbox_seatbelt_available", "rt_os_adv.c"),
        ("rt_bridge_exec", "rt_process.c"),
        ("rt_bridge_exec_arg", "rt_process.c"),
        ("command_run", "rt_process.c"),
        ("command_exec_capture", "rt_process.c"),
        ("alloc_track_start", "rt_alloc_track.c"),
        ("alloc_track_note", "rt_alloc_track.c"),
        ("alloc_track_end", "rt_alloc_track.c"),
        ("arc_alloc_i32", "rt_arc.c"),
        ("arc_alloc_string", "rt_arc.c"),
        ("arc_inc", "rt_arc.c"),
        ("arc_dec", "rt_arc.c"),
        ("arc_dec_i32", "rt_arc.c"),
        ("arc_dec_string", "rt_arc.c"),
        ("arc_get_i32", "rt_arc.c"),
        ("arc_get_string", "rt_arc.c"),
    ])
}

/// Nyra-facing `extern fn` name → C symbol in `stdlib/rt/`.
pub fn c_symbol_for(nyra_fn: &str) -> String {
    const ALIASES: &[(&str, &str)] = &[
        ("exists", "file_exists"),
        ("clone", "str_clone"),
        ("command_run_argv", "command_run"),
        ("command_exec_capture_argv", "command_exec_capture"),
        ("free", "heap_free"),
        ("strlen", "str_len"),
        ("strcat", "str_cat"),
        ("strcmp", "str_cmp"),
        ("Vec_str_new", "vec_str_new"),
        ("Vec_str_push", "vec_str_push"),
        ("Vec_str_free", "vec_str_free"),
        ("Vec_str_get", "vec_str_get"),
        ("Vec_str_len", "vec_str_len"),
        ("tcp_accept_async", "rt_tcp_accept_async"),
    ];
    for (name, sym) in ALIASES {
        if *name == nyra_fn {
            return sym.to_string();
        }
    }
    let map = symbol_module_map();
    if map.contains_key(nyra_fn) {
        return nyra_fn.to_string();
    }
    // `rt_*` stdlib FFI stubs use the same LLVM/C name (avoids collision with public `fn` wrappers).
    // User-declared `extern fn` (C libraries, bindgen): keep the symbol as written.
    nyra_fn.to_string()
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeProfile {
    pub symbols: BTreeSet<String>,
}

impl RuntimeProfile {
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    pub fn needs_pthread(&self) -> bool {
        self.modules().iter().any(|m| {
            matches!(
                *m,
                "rt_spawn.c" | "rt_channel.c" | "rt_sync.c" | "rt_async.c" | "rt_net.c" | "rt_os_adv.c"
                    | "rt_parallel.c" | "rt_race.c" | "rt_io_pool.c" | "rt_shm.c"
            )
        })
    }

    pub fn needs_openssl(&self) -> bool {
        self.symbols.iter().any(|s| {
            s.starts_with("tls_")
                || s.starts_with("ws_")
                || s.starts_with("rt_tls_")
                || s.starts_with("rsa_")
                || s.starts_with("x509_")
        })
    }

    pub fn needs_libm(&self) -> bool {
        self.symbols.iter().any(|s| {
            s == "sin_f64" || s == "cos_f64" || s == "atan2_f64" || s == "tan_f64"
        })
    }

    pub fn needs_zlib(&self) -> bool {
        self.symbols.iter().any(|s| {
            s == "gzip_file"
                || s == "gunzip_file"
                || s == "gzip_compress_hex"
                || s == "gzip_decompress_hex"
                || s == "flate_compress_hex"
                || s == "flate_decompress_hex"
        })
    }

    pub fn modules(&self) -> BTreeSet<&'static str> {
        let map = symbol_module_map();
        let mut mods = BTreeSet::new();
        for sym in &self.symbols {
            if let Some(m) = map.get(sym.as_str()) {
                mods.insert(*m);
            }
        }
        if self.symbols.contains("spawn") {
            mods.insert("rt_spawn.c");
        }
        if mods.contains("rt_async.c") {
            mods.insert("rt_spawn.c");
            // rt_async.c calls io_uring_* on Linux via extern; link the implementation unit.
            mods.insert("rt_io_uring.c");
        }
        if mods.contains("rt_http.c") {
            mods.insert("rt_net.c");
        }
        if mods.contains("rt_tls.c") {
            mods.insert("rt_net.c");
        }
        if self.symbols.contains("tcp_accept_async") {
            mods.insert("rt_async.c");
        }
        // rt_strings.c defines str_split → vec_str_* (whole .c unit is linked).
        if mods.contains("rt_strings.c") {
            mods.insert("rt_vec.c");
        }
        // rt_args.c defines vec_str_from_argv → vec_str_* (whole .c unit is linked).
        if mods.contains("rt_args.c") {
            mods.insert("rt_vec.c");
        }
        // rt_array.c debug formatters call str_cat / i32_to_string / f64_to_string.
        if mods.contains("rt_array.c") {
            mods.insert("rt_strings.c");
        }
        if mods.contains("rt_task_pool.c") {
            mods.insert("rt_parallel.c");
        }
        if mods.contains("rt_parallel.c") {
            mods.insert("rt_task_pool.c");
        }
        if mods.contains("rt_progress.c") {
            mods.insert("rt_io.c");
        }
        if mods.contains("rt_aes.c") {
            mods.insert("rt_crypto.c");
            mods.insert("aes_core.c");
        }
        if mods.contains("rt_websocket.c") {
            mods.insert("rt_net.c");
            mods.insert("rt_tls.c");
        }
        if mods.contains("rt_bench.c") {
            mods.insert("rt_io.c");
        }
        mods
    }

    /// Platform-aware runtime modules (e.g. Windows `rt_async.c` needs `rt_net.c` for Winsock).
    pub fn modules_for_target(&self, target: &str) -> BTreeSet<&'static str> {
        let mut mods = self.modules();
        if link_target_is_windows(target) && mods.contains("rt_async.c") {
            mods.insert("rt_net.c");
        }
        mods
    }

    pub fn uses_ws2_32(&self, target: &str) -> bool {
        self.modules_for_target(target).contains("rt_net.c")
    }
}

/// True when linking for Windows (`--target` empty uses the host OS at link time).
fn link_target_is_windows(target: &str) -> bool {
    if target.to_ascii_lowercase().contains("windows") {
        return true;
    }
    target.is_empty() && std::env::consts::OS == "windows"
}

pub fn stdlib_rt_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../stdlib/rt")
}

/// True when this binary was built from a Nyra source tree with `stdlib/rt/` present.
pub fn repo_stdlib_rt_available() -> bool {
    let rt_dir = stdlib_rt_dir();
    if !rt_dir.is_dir() {
        return false;
    }
    let probe = rt_dir.join("rt_io.c");
    runtime_module_has_symbols(&probe, &["stdin_read_line"])
}

fn installed_rt_common_is_stale(rt_dir: &Path) -> bool {
    let path = rt_dir.join("rt_common.h");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return false;
    };
    text.contains("clock_gettime(CLOCK_MONOTONIC")
        && !text.contains("#include <time.h>")
}

pub fn resolve_runtime_modules(profile: &RuntimeProfile, target: &str) -> Result<Vec<PathBuf>, String> {
    if target.contains("wasm") {
        return resolve_wasi_runtime(profile);
    }
    let rt_dir = stdlib_rt_dir();
    let mut paths = Vec::new();
    for mod_name in profile.modules_for_target(target) {
        let p = rt_dir.join(mod_name);
        if !p.is_file() {
            return Err(format!("Runtime module not found: {}", p.display()));
        }
        paths.push(p);
    }
    Ok(paths)
}

fn resolve_wasi_runtime(profile: &RuntimeProfile) -> Result<Vec<PathBuf>, String> {
    if profile.is_empty() {
        return Ok(Vec::new());
    }
    let rt_dir = wasi_rt_dir();
    let mods: Vec<_> = profile.modules().into_iter().collect();
    if mods.is_empty() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    if mods.contains(&"rt_io.c") {
        let common = rt_dir.join("rt_common.c");
        if common.is_file() {
            paths.push(common);
        }
    }
    for mod_name in mods {
        let p = rt_dir.join(mod_name);
        if !p.is_file() {
            let wasi = wasi_runtime_path();
            if wasi.is_file() {
                return Ok(vec![wasi]);
            }
            return Err(format!("WASI runtime module not found: {}", p.display()));
        }
        paths.push(p);
    }
    Ok(paths)
}

pub fn wasi_rt_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../stdlib/rt_wasi")
}

pub fn wasi_runtime_path() -> PathBuf {
    const WASI_REL: &str = "share/stdlib/nyra_rt_wasi.c";
    if let Ok(home) = std::env::var("NYRA_HOME") {
        let p = PathBuf::from(home).join(WASI_REL);
        if p.is_file() {
            return p;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(bin_dir) = exe.parent() {
            if let Some(root) = bin_dir.parent() {
                let p = root.join(WASI_REL);
                if p.is_file() {
                    return p;
                }
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../stdlib/nyra_rt_wasi.c")
}

pub fn legacy_runtime_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../stdlib/nyra_rt.c")
}

pub fn runtime_dir_from_install() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("NYRA_HOME") {
        let p = PathBuf::from(home).join("share/stdlib/rt");
        if p.is_dir() {
            return Some(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(bin_dir) = exe.parent() {
            if let Some(root) = bin_dir.parent() {
                let p = root.join("share/stdlib/rt");
                if p.is_dir() {
                    return Some(p);
                }
            }
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join(".nyra/share/stdlib/rt");
        if p.is_dir() {
            return Some(p);
        }
    }
    None
}

fn c_symbol_defined_in_source(text: &str, sym: &str) -> bool {
    let needle = format!("{sym}(");
    for (i, _) in text.match_indices(&needle) {
        if i == 0 {
            return true;
        }
        let prev = text.as_bytes()[i - 1];
        if !prev.is_ascii_alphanumeric() && prev != b'_' {
            return true;
        }
    }
    false
}

fn runtime_module_has_symbols(path: &Path, symbols: &[&str]) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    symbols
        .iter()
        .all(|sym| c_symbol_defined_in_source(&text, sym))
}

pub fn resolve_runtime_modules_installed(
    profile: &RuntimeProfile,
    target: &str,
) -> Result<Vec<PathBuf>, String> {
    if target.contains("wasm") {
        return resolve_wasi_runtime(profile);
    }
    if profile.is_empty() {
        return Ok(Vec::new());
    }
    // Dev tree / CI: prefer in-repo `stdlib/rt/` over `~/.nyra` (avoids stale installed copies).
    if repo_stdlib_rt_available() {
        return resolve_runtime_modules(profile, target);
    }
    if let Some(rt_dir) = runtime_dir_from_install() {
        let map = symbol_module_map();
        let mut symbols_by_mod: HashMap<&'static str, Vec<&str>> = HashMap::new();
        for sym in &profile.symbols {
            if let Some(mod_name) = map.get(sym.as_str()) {
                symbols_by_mod.entry(mod_name).or_default().push(sym.as_str());
            }
        }

        let mut paths = Vec::new();
        let mut stale = installed_rt_common_is_stale(&rt_dir);
        for mod_name in profile.modules_for_target(target) {
            let p = rt_dir.join(mod_name);
            if !p.is_file() {
                stale = true;
                break;
            }
            let needed = symbols_by_mod.get(mod_name).map(|v| v.as_slice()).unwrap_or(&[]);
            if !runtime_module_has_symbols(&p, needed) {
                stale = true;
                break;
            }
            if stale {
                break;
            }
            if let Ok(text) = std::fs::read_to_string(&p) {
                let needs_refresh = if target.to_ascii_lowercase().contains("windows") {
                    match mod_name {
                        "rt_net.c" => !text.contains("nyra_winsock_ensure"),
                        "rt_spawn.c" => !text.contains("CreateThread"),
                        "rt_channel.c" => !text.contains("InitializeConditionVariable"),
                        "rt_async.c" => !text.contains("SleepConditionVariableCS"),
                        "rt_strings.c" => !text.contains("str_replacen("),
                        _ => false,
                    }
                } else {
                    match mod_name {
                        "rt_net.c" => {
                            text.contains("pthread_create") && !text.contains("pthread.h")
                        }
                        "rt_strings.c" => !text.contains("str_replacen("),
                        "rt_async.c" => text.contains("static int async_future_done"),
                        _ => false,
                    }
                };
                if needs_refresh {
                    stale = true;
                    break;
                }
            }
            paths.push(p);
        }
        if !stale {
            return Ok(paths);
        }
        // Installed stdlib missing modules or symbols — use repo `stdlib/rt/`.
        return resolve_runtime_modules(profile, target);
    }
    resolve_runtime_modules(profile, target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_stdlib_rt_available_in_workspace() {
        assert!(repo_stdlib_rt_available());
    }

    #[test]
    fn c_symbol_for_strips_nyra_prefix_convention() {
        assert_eq!(c_symbol_for("strlen"), "str_len");
        assert_eq!(c_symbol_for("read_file"), "read_file");
        assert_eq!(c_symbol_for("exists"), "file_exists");
        assert_eq!(c_symbol_for("free"), "heap_free");
        assert_eq!(c_symbol_for("ClearBackground"), "ClearBackground");
    }

    #[test]
    fn async_on_windows_pulls_rt_net() {
        let mut p = RuntimeProfile::default();
        p.symbols.insert("async_promise_new".into());
        let mods = p.modules_for_target("x86_64-pc-windows-gnu");
        assert!(mods.contains("rt_async.c"));
        assert!(mods.contains("rt_net.c"));
        let mods_linux = p.modules_for_target("x86_64-unknown-linux-gnu");
        assert!(mods_linux.contains("rt_async.c"));
        assert!(!mods_linux.contains("rt_net.c"));
    }

    #[test]
    fn async_empty_target_uses_host_os_for_windows_modules() {
        let mut p = RuntimeProfile::default();
        p.symbols.insert("async_promise_new".into());
        let mods = p.modules_for_target("");
        if std::env::consts::OS == "windows" {
            assert!(
                mods.contains("rt_net.c"),
                "empty target on Windows host must link rt_net.c for async"
            );
        } else {
            assert!(!mods.contains("rt_net.c"));
        }
    }

    #[test]
    fn link_target_is_windows_empty_means_host() {
        if std::env::consts::OS == "windows" {
            assert!(super::link_target_is_windows(""));
        } else {
            assert!(!super::link_target_is_windows(""));
        }
        assert!(super::link_target_is_windows("x86_64-pc-windows-gnu"));
        assert!(!super::link_target_is_windows("x86_64-unknown-linux-gnu"));
    }

    #[test]
    fn async_module_pulls_spawn_for_capture_helper() {
        let mut p = RuntimeProfile::default();
        p.symbols.insert("async_promise_new".into());
        let mods = p.modules();
        assert!(mods.contains("rt_async.c"));
        assert!(mods.contains("rt_spawn.c"));
        assert!(mods.contains("rt_io_uring.c"));
    }

    #[test]
    fn sqlite_open_pulls_sqlite_module() {
        let mut p = RuntimeProfile::default();
        p.symbols.insert("sqlite_open".into());
        assert!(p.modules().contains("rt_sqlite.c"));
    }

    #[test]
    fn spawn_pulls_spawn_module() {
        let mut p = RuntimeProfile::default();
        p.symbols.insert("spawn".into());
        assert!(p.modules().contains("rt_spawn.c"));
        assert!(p.modules().contains("rt_async.c"));
    }

    #[test]
    fn rt_args_module_pulls_vec_for_argv_helper() {
        let mut p = RuntimeProfile::default();
        p.symbols.insert("rt_args_init".into());
        let mods = p.modules();
        assert!(mods.contains("rt_args.c"));
        assert!(mods.contains("rt_vec.c"));
    }

    #[test]
    fn strings_module_pulls_vec_for_internal_split() {
        let mut p = RuntimeProfile::default();
        p.symbols.insert("str_len".into());
        let mods = p.modules();
        assert!(mods.contains("rt_strings.c"));
        assert!(mods.contains("rt_vec.c"));
    }

    #[test]
    fn async_future_done_maps_to_rt_async() {
        let mut p = RuntimeProfile::default();
        p.symbols.insert("async_future_done".into());
        assert!(p.modules().contains("rt_async.c"));
    }

    #[test]
    fn runtime_module_symbol_match_avoids_prefix_false_positive() {
        let dir = std::env::temp_dir().join(format!("nyra_rt_sym_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rt_io.c");
        std::fs::write(&path, "char *nyra_stdin_read_line(const char *p) { return 0; }\n").unwrap();
        assert!(!runtime_module_has_symbols(&path, &["stdin_read_line"]));
        std::fs::write(&path, "char *stdin_read_line(const char *p) { return 0; }\n").unwrap();
        assert!(runtime_module_has_symbols(&path, &["stdin_read_line"]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn aes_module_pulls_aes_core_and_crypto() {
        let mut p = RuntimeProfile::default();
        p.symbols.insert("aes_cbc_encrypt_hex".into());
        let mods = p.modules();
        assert!(mods.contains("rt_aes.c"));
        assert!(mods.contains("aes_core.c"));
        assert!(mods.contains("rt_crypto.c"));
        assert!(!p.needs_openssl());
    }

    #[test]
    fn sha512_does_not_require_openssl() {
        let mut p = RuntimeProfile::default();
        p.symbols.insert("sha512_hex".into());
        assert!(p.modules().contains("rt_crypto_sha512.c"));
        assert!(!p.needs_openssl());
    }

    #[test]
    fn tls_and_ws_still_require_openssl() {
        let mut p = RuntimeProfile::default();
        p.symbols.insert("tls_available".into());
        assert!(p.needs_openssl());
        p.symbols.clear();
        p.symbols.insert("ws_connect".into());
        assert!(p.needs_openssl());
    }

    #[test]
    fn installed_resolve_falls_back_when_symbols_renamed() {
        let mut p = RuntimeProfile::default();
        p.symbols.insert("stdin_read_line".into());
        p.symbols.insert("stdout_flush".into());
        let paths = resolve_runtime_modules_installed(&p, "").unwrap();
        assert!(!paths.is_empty(), "expected runtime modules");
        let io = paths
            .iter()
            .find(|p| p.ends_with("rt_io.c"))
            .expect("rt_io.c");
        let text = std::fs::read_to_string(io).unwrap();
        assert!(
            text.contains("stdin_read_line("),
            "resolved rt_io.c must export stdin_read_line: {}",
            io.display()
        );
    }
}
