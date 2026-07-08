package main

import (
	"fmt"
	"runtime"
)


func main() {

	const mod = 1000000007
	base := "benchmark-substring-padding-value"
	var acc int64 = 0
	for i := 0; i < 100000; i++ {
		start := i % 10
		part := base[start : start+8]
		acc = (acc + int64(len(part))) % mod
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
