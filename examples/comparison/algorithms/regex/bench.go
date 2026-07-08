package main

import (
	"fmt"
	"runtime"
	"regexp"
)


func main() {

	const mod = 1000000007
	re := regexp.MustCompile("bench_[0-9]+")
	text := "prefix bench_12345 suffix"
	var acc int64 = 0
	for i := 0; i < 100000; i++ {
		if re.MatchString(text) {
			acc = (acc + 1) % mod
		}
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
