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
		v = append(v, i%997)
		acc = (acc + int64(len(v))) % mod
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
