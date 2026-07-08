fn main() -> void {
    spit("/tmp/nyra_docs_marker.txt", "x")
    print(ls("/tmp").len() > 0)
    print(rm("/tmp/nyra_docs_marker.txt"))
}
