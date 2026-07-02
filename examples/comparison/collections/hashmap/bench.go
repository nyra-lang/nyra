package main

import (
	"fmt"
	"runtime"
)


func main() {

	const mod = 1000000007
	m := make(map[int]int, 10000)
	var acc int64 = 0
	for i := 0; i < 200000; i++ {
		k := i % 10000
		m[k] = i
		acc = (acc + int64(m[k])) % mod
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
