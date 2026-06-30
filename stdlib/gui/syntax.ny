extern fn strlen(s: &string) -> i32
extern fn strcmp(a: &string, b: &string) -> i32
extern fn strstr_pos(hay: &string, needle: &string) -> i32
extern fn substring(s: &string, start: i32, len: i32) -> string
extern fn char_at(s: &string, i: i32) -> i32

const SYNTAX_PLAIN = 0
const SYNTAX_KEYWORD = 1
const SYNTAX_STRING = 2
const SYNTAX_COMMENT = 3
const SYNTAX_NUMBER = 4

fn Syntax_is_keyword(word: string) -> i32 {
    if strcmp(word, "fn") == 0 { return 1 }
    if strcmp(word, "let") == 0 { return 1 }
    if strcmp(word, "mut") == 0 { return 1 }
    if strcmp(word, "if") == 0 { return 1 }
    if strcmp(word, "else") == 0 { return 1 }
    if strcmp(word, "while") == 0 { return 1 }
    if strcmp(word, "for") == 0 { return 1 }
    if strcmp(word, "return") == 0 { return 1 }
    if strcmp(word, "struct") == 0 { return 1 }
    if strcmp(word, "import") == 0 { return 1 }
    if strcmp(word, "match") == 0 { return 1 }
    if strcmp(word, "break") == 0 { return 1 }
    if strcmp(word, "continue") == 0 { return 1 }
    return 0
}

fn Syntax_line_kind(line: string) -> i32 {
    let n = strlen(line)
    let mut i = 0
    while i < n {
        let c = char_at(line, i)
        if c != 32 && c != 9 {
            if c == 47 && i + 1 < n && char_at(line, i + 1) == 47 {
                return SYNTAX_COMMENT
            }
            break
        }
        i = i + 1
    }
    return SYNTAX_PLAIN
}

fn Syntax_token_kind(token: string) -> i32 {
    if strlen(token) == 0 {
        return SYNTAX_PLAIN
    }
    if char_at(token, 0) == 34 {
        return SYNTAX_STRING
    }
    if char_at(token, 0) >= 48 && char_at(token, 0) <= 57 {
        return SYNTAX_NUMBER
    }
    if Syntax_is_keyword(token) == 1 {
        return SYNTAX_KEYWORD
    }
    return SYNTAX_PLAIN
}
