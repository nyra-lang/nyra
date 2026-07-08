package main

import (
	"fmt"
	"runtime"
)


func main() {

	const mod = 1000000007
	v := make([]int, 0, 50000)
	var acc int64 = 0
	for i := 0; i < 50000; i++ {
		t := 50000 - i
		v = append(v, t%997)
	}
	n := len(v)
	for gap := n / 2; gap > 0; gap /= 2 {
		for j := gap; j < n; j++ {
			key := v[j]
			k := j
			for k >= gap && v[k-gap] > key {
				k -= gap
			}
		}
	}
	for _, x := range v {
		acc = (acc + int64(x)) % mod
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
