// [contrib-dev:file_is_symlink:fs_file]
import "stdlib/testing.ny"
import "stdlib/fs/file.ny"

test fn test_file_is_symlink() {
    assert_eq(file_is_symlink("/nonexistent-link-xyz"), 0)
}
// [/contrib-dev:file_is_symlink:fs_file]
