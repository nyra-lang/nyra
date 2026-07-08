// run-stdout: ok
import "stdlib/pkg/git_fetch.ny"

fn main() {
    let url = GitFetch_github_tarball_url(
        "https://github.com/nyra-lang/nyra.git",
        "main"
    )
    if strstr_pos(url, "github.com/nyra-lang/nyra/archive/refs/heads/main.tar.gz") < 0 {
        print("bad url")
        return
    }
    print("ok")
}
