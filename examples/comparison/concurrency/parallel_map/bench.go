package main

import (
	"fmt"
	"runtime"
	"sync"
)


func main() {

	const mod = 1000000007
	var wg sync.WaitGroup
	for i := 0; i < 200000; i++ {
		wg.Add(1)
		go func(x int) {
			defer wg.Done()
			runtime.KeepAlive((x % 997) * 31)
		}(i)
	}
	wg.Wait()
	var acc int64 = 0
	for i := 0; i < 200000; i++ {
		acc = (acc + int64((i%997)*31)) % mod
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
