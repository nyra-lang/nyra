// Language/stdlib gaps fixed for Games suite (zero-types).

const COLS = 10
const ROWS = 5

fn test_array_repeat_mul() {
    let grid = [0; COLS * ROWS]
    if grid[0] != 0 {
        print("fail repeat mul")
    }
}

fn test_continue_stmt() {
    let mut i = 0
    let mut sum = 0
    while i < 5 {
        i = i + 1
        if i == 3 {
            continue
        }
        sum = sum + i
    }
    if sum != 12 {
        print("fail continue", sum)
    }
}

fn test_bool_int_cmp() {
    let light = 1 % 2 == 0
    if light == false {
        print("ok bool int")
    }
}

fn test_i32_to_f64_assign() {
    let mut y = 0.0
    let dt = 0.016
    y = y + (120.0 * dt)
    if y < 1.0 {
        print("fail f64 assign")
    }
}

fn test_neg_array_repeat() {
    let row = [-1, 2, -3]
    let fill = [-5; 4]
    if fill[0] != -5 {
        print("fail neg repeat")
    }
    if row[2] != -3 {
        print("fail neg literal")
    }
}

fn test_trig() {
    let s = sin(0.0)
    let c = cos(0.0)
    if s > 0.01 || c < 0.99 {
        print("fail trig", s, c)
    }
}

fn test_random_f64() {
    let r = random_f64()
    if r < 0.0 || r >= 1.0 {
        print("fail random_f64", r)
    }
}

fn Grid_adjacent(mines, idx, width) {
    let row = idx / width
    let col = idx % width
    let mut count = 0
    let mut dr = -1
    while dr <= 1 {
        let mut dc = -1
        while dc <= 1 {
            if dr != 0 || dc != 0 {
                let nr = row + dr
                let nc = col + dc
                if nr >= 0 && nc >= 0 && nr < width && nc < width {
                    if mines[nr * width + nc] != 0 {
                        count = count + 1
                    }
                }
            }
            dc = dc + 1
        }
        dr = dr + 1
    }
    return count
}

fn test_array_param_inference() {
    let mines = [1, 0, 0, 1]
    let n = Grid_adjacent(mines, 1, 2)
    if n != 2 {
        print("fail array infer", n)
    }
}

fn main() {
    test_array_repeat_mul()
    test_continue_stmt()
    test_bool_int_cmp()
    test_i32_to_f64_assign()
    test_neg_array_repeat()
    test_trig()
    test_random_f64()
    test_array_param_inference()
    print("games_gaps ok")
}
