package main

import (
	"fmt"
	"runtime"
)


func main() {

	const n = 500000
	const mod = 1000000007
	type pair struct{ a, b int64 }
	var acc int64 = 0
	for i := int64(0); i < n; i++ {
		p := pair{i % 1000, (i * 7) % 1000}
		acc = (acc + p.a + p.b) % mod
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
