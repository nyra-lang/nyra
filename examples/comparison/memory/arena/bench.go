package main

import (
	"fmt"
	"runtime"
)


func main() {

	const n = 500000
	const mod = 1000000007
	var acc int64 = 0
	var bump int64 = 0
	for i := int64(0); i < n; i++ {
		bump = (bump + 16) % 67108864
		acc = (acc + bump + i) % mod
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
