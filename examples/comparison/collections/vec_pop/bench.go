package main

import (
	"fmt"
	"runtime"
)


func main() {

	const mod = 1000000007
	v := make([]int, 0, 500000)
	var acc int64 = 0
	for i := 0; i < 500000; i++ {
		v = append(v, i)
	}
	for len(v) > 0 {
		acc = (acc + int64(v[len(v)-1])) % mod
		v = v[:len(v)-1]
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
