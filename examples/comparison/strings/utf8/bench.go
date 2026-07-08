package main

import (
	"fmt"
	"runtime"
)


func main() {

	const mod = 1000000007
	s := "Nyra_utf8_bench_mix"
	var acc int64 = 0
	for i := 0; i < 100000; i++ {
		for j := 0; j < len(s); j++ {
			acc = (acc + int64(s[j])) % mod
		}
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
