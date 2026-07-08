import "for_in_generic.typed.ny" as g

fn test_comptime_for_in_generic() {
    if g::TOTAL != 10 {
        print("fail total", g::TOTAL)
    }
}
