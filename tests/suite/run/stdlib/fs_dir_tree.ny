// run-stdout: ok
import "stdlib/fs.ny"

fn main() {
    let base = ".nyra_test_fs_dir"
    if exists(base) == 1 {
        remove_dir_all(base)
    }
    if create_dir_all(join_path(join_path(base, "a"), "b/c")) != 0 {
        print("fail mkdir")
        return
    }
    if is_dir(join_path(join_path(join_path(base, "a"), "b"), "c")) == 0 {
        print("fail isdir")
        return
    }
    write_file(join_path(join_path(join_path(base, "a"), "b"), "x.txt"), "hi")
    let dst = strcat(base, "_copy")
    if exists(dst) == 1 {
        remove_dir_all(dst)
    }
    if copy_dir(base, dst) != 0 {
        print("fail copy")
        return
    }
    if file_exists(join_path(join_path(join_path(dst, "a"), "b"), "x.txt")) == 0 {
        print("fail verify")
        return
    }
    remove_dir_all(base)
    remove_dir_all(dst)
    print("ok")
}
