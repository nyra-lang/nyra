package main

import (
	"fmt"
	"runtime"
)

const (
	tableLen    = 64
	buildIters  = 8000
	sumRounds   = 8
	mod   int64 = 1000000007
)

func mix(n int64) int64 {
	a := n * 100003
	b := n * n
	return (a + b*31 + 997) % mod
}

func main() {
	table := make([]int64, tableLen)
	for i := 0; i < tableLen; i++ {
		var v int64
		for k := 0; k < buildIters; k++ {
			v = (v + mix(int64(i+k))) % mod
		}
		table[i] = v
	}
	var acc int64
	for r := 0; r < sumRounds; r++ {
		for j := 0; j < tableLen; j++ {
			acc = (acc + table[j]) % mod
		}
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
