package main

import (
	"fmt"
	"runtime"
)


func main() {

	const mod = 1000000007
	var acc int64 = 0
	for i := 0; i < 50000; i++ {
		t := 50000 - i
		acc = (acc + int64(t%997)) % mod
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
