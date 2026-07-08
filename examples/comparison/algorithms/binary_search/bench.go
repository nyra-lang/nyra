package main

import (
	"fmt"
	"runtime"
)


func main() {

	const mod = 1000000007
	n := 50000
	lo, hi := 0, n
	target := n / 3
	probes := 0
	var acc int64 = 0
	for lo < hi && probes < 32 {
		mid := (lo + hi) / 2
		if mid < target {
			lo = mid + 1
		} else {
			hi = mid
		}
		acc = (acc + int64(mid)) % mod
		probes++
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
