import "../strings.ny"
import "../strings/ops.ny"
import "../fs.ny"
import "../fs/dir.ny"
import "../archive/tar.ny"
import "../compress/gzip.ny"
import "stdlib/http/download.ny"

fn GitFetch_strip_git_suffix(url: string) -> string {
    if str_ends_with(url, ".git") == 1 {
        return substring(url, 0, strlen(url) - 4)
    }
    return url
}

fn GitFetch_github_tarball_url(url: string, rev: string) -> string {
    let base = GitFetch_strip_git_suffix(url)
    if str_starts_with(base, "https://github.com/") == 0 {
        return ""
    }
    return strcat(
        strcat(strcat(base, "/archive/refs/heads/"), rev),
        ".tar.gz"
    )
}

fn GitFetch_single_root_dir(extract_dir: string) -> string {
    let entries = list_dir_entries(extract_dir)
    if entries.len() != 1 {
        return ""
    }
    let root = join_path(extract_dir, entries.get(0))
    if path_is_dir(root) == 0 {
        return ""
    }
    return root
}

fn GitFetch_cleanup_temp(tgz: string, tar_path: string, unpack: string) -> void {
    if file_exists(tgz) == 1 {
        remove_file(tgz)
    }
    if file_exists(tar_path) == 1 {
        remove_file(tar_path)
    }
    if file_exists(unpack) == 1 {
        remove_dir_all(unpack)
    }
}

fn GitFetch_http_tarball(url: string, rev: string, dest: string, cache_base: string) -> i32 {
    let tarball_url = GitFetch_github_tarball_url(url, rev)
    if strlen(tarball_url) == 0 {
        return -1
    }
    let tgz = join_path(cache_base, "_git_fetch.tgz")
    let tar_path = join_path(cache_base, "_git_fetch.tar")
    let unpack = join_path(cache_base, "_git_fetch_unpack")
    GitFetch_cleanup_temp(tgz, tar_path, unpack)
    create_dir_all(cache_base)
    if http_download_file(tarball_url, tgz) != 0 {
        GitFetch_cleanup_temp(tgz, tar_path, unpack)
        return -1
    }
    if gunzip_file(tgz, tar_path) != 0 {
        GitFetch_cleanup_temp(tgz, tar_path, unpack)
        return -1
    }
    create_dir_all(unpack)
    if tar_extract(tar_path, unpack) != 0 {
        GitFetch_cleanup_temp(tgz, tar_path, unpack)
        return -1
    }
    let root = GitFetch_single_root_dir(unpack)
    if strlen(root) == 0 {
        GitFetch_cleanup_temp(tgz, tar_path, unpack)
        return -1
    }
    if file_exists(dest) == 1 {
        remove_dir_all(dest)
    }
    create_dir_all(dest)
    let code = copy_dir_contents(root, dest)
    GitFetch_cleanup_temp(tgz, tar_path, unpack)
    return code
}
