package main

import (
	"fmt"
	"runtime"
	"strings"
)


func main() {

	const mod = 1000000007
	s := "foo-bar-baz-"
	var acc int64 = 0
	for i := 0; i < 100000; i++ {
		s = strings.Replace(s, "bar", "qux", 1)
		acc = (acc + int64(len(s))) % mod
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
