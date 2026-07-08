fn main() {
    print(cmd("true").run())
    let out = cmd("echo").arg("hi").output()
    print(out.code)
    print(out.stdout.len())
}
