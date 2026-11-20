package main

import (
	"fmt"
	"math/bits"
	"time"
)

// --- W1: Matrix GEMM 256x256 FP32 ---
func benchGEMM() float64 {
	n := 256
	a := make([]float32, n*n)
	b := make([]float32, n*n)
	c := make([]float32, n*n)
	for i := range a {
		a[i] = 0.5
		b[i] = 0.5
	}

	t0 := time.Now()
	for i := 0; i < n; i++ {
		for k := 0; k < n; k++ {
			aik := a[i*n+k]
			for j := 0; j < n; j++ {
				c[i*n+j] += aik * b[k*n+j]
			}
		}
	}
	elapsed := float64(time.Since(t0).Nanoseconds()) / 1000000.0
	if c[0] < 0 {
		fmt.Printf("checksum: %f\n", c[0])
	}
	return elapsed
}

// --- W2: BNN 1-Bit POPCOUNT GEMV (4096 rows x 4096 cols = 64 words) ---
func benchBNN() float64 {
	rows := 4096
	words := 64
	weights := make([]uint64, rows*words)
	act := make([]uint64, words)
	out := make([]int32, rows)

	for i := range act {
		act[i] = 0xAAAAAAAAAAAAAAAA
	}
	for i := range weights {
		weights[i] = 0x5555555555555555
	}

	t0 := time.Now()
	for r := 0; r < rows; r++ {
		var sum int32 = 0
		base := r * words
		for w := 0; w < words; w++ {
			x := weights[base+w] ^ act[w]
			sum += int32(bits.OnesCount64(x))
		}
		out[r] = sum
	}
	elapsed := float64(time.Since(t0).Nanoseconds()) / 1000000.0
	if out[0] < 0 {
		fmt.Printf("checksum: %d\n", out[0])
	}
	return elapsed
}

// --- W3: Quicksort (100,000 int64) ---
func quicksortI64(arr []int64, low, high int) {
	if low >= high {
		return
	}
	pivot := arr[high]
	i := low - 1
	for j := low; j < high; j++ {
		if arr[j] <= pivot {
			i++
			arr[i], arr[j] = arr[j], arr[i]
		}
	}
	arr[i+1], arr[high] = arr[high], arr[i+1]
	pi := i + 1

	quicksortI64(arr, low, pi-1)
	quicksortI64(arr, pi+1, high)
}

func benchSort() float64 {
	n := 100000
	arr := make([]int64, n)
	var seed uint64 = 42
	for i := 0; i < n; i++ {
		seed = (seed*1103515245 + 12345) & 0x7FFFFFFF
		arr[i] = int64(seed % 1000000)
	}

	t0 := time.Now()
	quicksortI64(arr, 0, n-1)
	elapsed := float64(time.Since(t0).Nanoseconds()) / 1000000.0

	if arr[0] < 0 {
		fmt.Printf("checksum: %d\n", arr[0])
	}
	return elapsed
}

// --- W4: Sieve of Eratosthenes (2,000,000) ---
func benchSieve() float64 {
	n := 2000000
	isPrime := make([]bool, n+1)
	for i := 2; i <= n; i++ {
		isPrime[i] = true
	}

	t0 := time.Now()
	for p := 2; p*p <= n; p++ {
		if isPrime[p] {
			for i := p * p; i <= n; i += p {
				isPrime[i] = false
			}
		}
	}

	count := 0
	for i := 2; i <= n; i++ {
		if isPrime[i] {
			count++
		}
	}
	elapsed := float64(time.Since(t0).Nanoseconds()) / 1000000.0

	if count == 0 {
		fmt.Printf("checksum: %d\n", count)
	}
	return elapsed
}

// --- W5: 3D Particle Kinematics (50,000 particles x 10 steps) ---
type Particle struct {
	x, y, z    float32
	vx, vy, vz float32
}

func benchPhysics() float64 {
	n := 50000
	p := make([]Particle, n)
	for i := 0; i < n; i++ {
		p[i].x = 0.0
		p[i].y = 100.0
		p[i].z = 0.0
		p[i].vx = 1.0
		p[i].vy = 0.0
		p[i].vz = 0.5
	}
	var dt float32 = 0.016
	var g float32 = -9.81

	t0 := time.Now()
	for step := 0; step < 10; step++ {
		for i := 0; i < n; i++ {
			p[i].x += p[i].vx * dt
			p[i].y += p[i].vy * dt
			p[i].z += p[i].vz * dt
			p[i].vy += g * dt
		}
	}
	elapsed := float64(time.Since(t0).Nanoseconds()) / 1000000.0

	if p[0].y < -9999.0 {
		fmt.Printf("checksum: %f\n", p[0].y)
	}
	return elapsed
}

// --- W6: JSON Scanner (100,000 runs) ---
func benchJSON() float64 {
	doc := []byte("{\"order_id\": 9876543, \"user_id\": 42109, \"symbol\": \"BTCUSD\", \"qty\": 1500, \"price\": 96500, \"status\": \"FILLED\"}")
	docLen := len(doc)
	key := []byte("\"price\": ")
	keyLen := len(key)

	t0 := time.Now()
	var sumPrice int64 = 0
	for r := 0; r < 100000; r++ {
		for i := 0; i <= docLen-keyLen; i++ {
			if doc[i] == '"' && doc[i+1] == 'p' {
				match := true
				for k := 0; k < keyLen; k++ {
					if doc[i+k] != key[k] {
						match = false
						break
					}
				}
				if match {
					var val int64 = 0
					k := i + keyLen
					for k < docLen && doc[k] >= '0' && doc[k] <= '9' {
						val = val*10 + int64(doc[k]-'0')
						k++
					}
					sumPrice += val
					break
				}
			}
		}
	}
	elapsed := float64(time.Since(t0).Nanoseconds()) / 1000000.0

	if sumPrice == 0 {
		fmt.Printf("checksum: %d\n", sumPrice)
	}
	return elapsed
}

func fibGo(n int64) int64 {
	if n <= 1 { return n }
	return fibGo(n-1) + fibGo(n-2)
}

func benchFib() float64 {
	t0 := time.Now()
	r := fibGo(35)
	el := float64(time.Since(t0).Nanoseconds()) / 1000000.0
	if r != 9227465 { fmt.Printf("checksum: %d\n", r) }
	return el
}

func benchCRC32() float64 {
	n := 16777216
	buf := make([]byte, n)
	var tbl [256]uint32
	for i := 0; i < 256; i++ {
		c := uint32(i)
		for k := 0; k < 8; k++ {
			if (c & 1) != 0 { c = (c >> 1) ^ 0xEDB88320 } else { c >>= 1 }
		}
		tbl[i] = c
	}
	for i := 0; i < n; i++ { buf[i] = byte((i*7 + 13) & 255) }
	t0 := time.Now()
	var crc uint32 = 0xFFFFFFFF
	for _, b := range buf { crc = (crc >> 8) ^ tbl[byte(crc)^b] }
	crc ^= 0xFFFFFFFF
	el := float64(time.Since(t0).Nanoseconds()) / 1000000.0
	if crc == 0 { fmt.Printf("checksum: %d\n", crc) }
	return el
}

func benchMandel() float64 {
	size := 512
	t0 := time.Now()
	var total int64
	for py := 0; py < size; py++ {
		ci := (float64(py)/512.0)*2.4 - 1.2
		for px := 0; px < size; px++ {
			cr := (float64(px)/512.0)*3.0 - 2.0
			zr, zi := 0.0, 0.0
			it := 0
			for it < 64 {
				zr2, zi2 := zr*zr, zi*zi
				if zr2+zi2 > 4.0 { break }
				zi = 2.0*zr*zi + ci
				zr = zr2 - zi2 + cr
				it++
			}
			total += int64(it)
		}
	}
	el := float64(time.Since(t0).Nanoseconds()) / 1000000.0
	if total == 0 { fmt.Printf("checksum: %d\n", total) }
	return el
}

func benchGCChurn() float64 {
	t0 := time.Now()
	var acc int64
	for i := int64(0); i < 1000000; i++ {
		b := make([]int64, 0, 8)
		b = append(b, i&255)
		acc += b[0]
	}
	el := float64(time.Since(t0).Nanoseconds()) / 1000000.0
	if acc == 0 { fmt.Printf("checksum: %d\n", acc) }
	return el
}

func main() {
	tGemm := benchGEMM()
	tBnn := benchBNN()
	tSort := benchSort()
	tSieve := benchSieve()
	tPhys := benchPhysics()
	tJson := benchJSON()
	tFib := benchFib()
	tCrc := benchCRC32()
	tMandel := benchMandel()
	tGc := benchGCChurn()

	fmt.Printf("{\"lang\": \"Go (go 1.26)\", \"gemm\": %.2f, \"bnn\": %.3f, \"sort\": %.2f, \"sieve\": %.2f, \"physics\": %.2f, \"json\": %.2f, \"fib\": %.2f, \"crc32\": %.2f, \"mandel\": %.2f, \"gc_churn\": %.2f}\n",
		tGemm, tBnn, tSort, tSieve, tPhys, tJson, tFib, tCrc, tMandel, tGc)
}
