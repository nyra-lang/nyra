fn main() {
    spit("/tmp/nyra_docs_slurp.txt", "hello")
    print(slurp("/tmp/nyra_docs_slurp.txt"))
    spit_append("/tmp/nyra_docs_slurp.txt", "!")
    print(slurp("/tmp/nyra_docs_slurp.txt"))
    rm("/tmp/nyra_docs_slurp.txt")
}
