package main

import (
	"fmt"
	"runtime"
)


func main() {

	const n = 500000
	const mod = 1000000007
	ch := make(chan int, 128)
	go func() {
		for j := 0; j < n; j++ { ch <- j }
	}()
	var acc int64 = 0
	for i := 0; i < n; i++ {
		acc = (acc + int64(<-ch)) % mod
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
