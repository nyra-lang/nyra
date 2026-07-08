import "stdlib/strconv/mod.ny"
import "stdlib/flag/mod.ny"
import "stdlib/bufio/mod.ny"
import "stdlib/context/mod.ny"
import "stdlib/sync/mutex.ny"
import "stdlib/encoding/csv.ny"
import "stdlib/mime/mod.ny"

fn main() {
    print(atoi("42"))
    print(itoa(99))
    print(format_i32(7))
    print(format_f64(parse_f64("3.14")))

    let mut set = FlagSet_new("demo", " [options]")
    set = Flag_parse(set)
    if set.help() != 0 {
        Flag_print_usage(set)
        return
    }

    let mut sc = Scanner_new("a\nb\nc")
    sc = Scanner_scan(sc)
    print(Scanner_text(sc))

    let ctx = Context_with_timeout(Context_background(), 100)
    print(context_done(ctx))

    let mut mu = Mutex_new()
    mu = mu.lock()
    mu = mu.unlock()

    let mut row = StrVec_new()
    row = row.push("name")
    row = row.push("value")
    print(csv_format_row(row))

    print(mime_content_type_multipart("abc123"))
}
