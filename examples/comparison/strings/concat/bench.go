package main

import (
	"fmt"
	"runtime"
)


func main() {

	const mod = 1000000007
	var acc int64 = 0
	s := "a"
	for i := 0; i < 100000; i++ {
		s += "x"
		acc = (acc + int64(len(s))) % mod
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
