fn main() {
    print(form().append("a", "1").append("b", "2").urlencoded())
    print(params().append("q", "nyra").to_string())
}
