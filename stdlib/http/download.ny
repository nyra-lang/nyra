extern fn http_download_file(url: string, path: string) -> i32

fn download_file(url: string, path: string) -> i32 {
    return http_download_file(url, path)
}
