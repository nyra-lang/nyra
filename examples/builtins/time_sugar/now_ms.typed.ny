fn main() -> void {
    let t0 = now()
    ms(1).sleep()
    print(if t0.elapsed_ms() >= 0 { 1 } else { 0 })
}
