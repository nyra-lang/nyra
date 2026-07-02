package main

import (
	"fmt"
	"runtime"
)


func main() {

	const n = 5000
	const mod = 1000000007
	for i := 0; i < n; i++ {
		go func(x int) { runtime.KeepAlive(x) }(i)
	}
	var acc int64 = int64(n) % mod
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
