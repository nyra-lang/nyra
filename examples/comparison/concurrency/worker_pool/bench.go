package main

import (
	"fmt"
	"runtime"
)


func main() {

	const mod = 1000000007
	const total = 500000
	const workers = 4
	jobs := make(chan int, 128)
	results := make(chan int, 128)
	for w := 0; w < workers; w++ {
		go func() {
			for {
				job := <-jobs
				if job < 0 {
					return
				}
				results <- (job * 31) % 997
			}
		}()
	}
	for i := 0; i < total; i++ {
		jobs <- i
	}
	for s := 0; s < workers; s++ {
		jobs <- -1
	}
	var acc int64 = 0
	for g := 0; g < total; g++ {
		acc = (acc + int64(<-results)) % mod
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
