package main

import (
	"fmt"
	"runtime"
	"strings"
)


func main() {

	const mod = 1000000007
	hay := "alpha,beta,gamma,delta,epsilon"
	var acc int64 = 0
	for i := 0; i < 100000; i++ {
		pos := strings.Index(hay, ",")
		part := hay[:pos]
		acc = (acc + int64(len(part)) + int64(pos)) % mod
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
