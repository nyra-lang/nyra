// parallel any / find / all — early-exit parallel search (fork-join + atomic cancel).
allow_extended

fn main() {
    let any_hit = parallel any for i in 0..1000 {
        i == 42
    }
    print(any_hit)

    let idx = parallel find for i in 0..1000 {
        i == 42
    }
    print(idx)

    let all_pos = parallel all for i in 1..10 {
        i > 0
    }
    print(all_pos)
}
