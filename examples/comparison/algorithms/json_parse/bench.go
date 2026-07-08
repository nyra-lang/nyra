package main

import (
	"fmt"
	"runtime"
	"strings"
)

func jsonGetI32(doc, key string) int64 {
	needle := "\"" + key + "\":"
	idx := strings.Index(doc, needle)
	if idx < 0 {
		return 0
	}
	p := idx + len(needle)
	for p < len(doc) && (doc[p] == ' ' || doc[p] == '\t') {
		p++
	}
	sign := int64(1)
	if p < len(doc) && doc[p] == '-' {
		sign = -1
		p++
	}
	var v int64
	for p < len(doc) && doc[p] >= '0' && doc[p] <= '9' {
		v = v*10 + int64(doc[p]-'0')
		p++
	}
	return v * sign
}

func main() {

	const mod = 1000000007
	doc := `{"id": 42, "value": 997, "nested": {"x": 7}}`
	var acc int64 = 0
	for i := 0; i < 100000; i++ {
		acc = (acc + jsonGetI32(doc, "value")) % mod
		acc = (acc + jsonGetI32(doc, "id")) % mod
	}
	runtime.KeepAlive(acc)
	fmt.Println(acc)
}
