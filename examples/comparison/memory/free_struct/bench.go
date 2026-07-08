package main

import (
	"fmt"
	"runtime"
)


func main() {

	const n = 500000
	const mod = 1000000007
	var acc int64 = 0
	for i := int64(0); i < n; i++ {
		p := make([]byte, 16)
		acc = (acc + i) % mod
		runtime.KeepAlive(p)
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
