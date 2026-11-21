package main

import (
	"fmt"
	"math"
	"time"
)

func benchNBody() (float64, int64) {
	const n, steps = 256, 40
	const dt = 0.01
	px, py, pz := make([]float64, n), make([]float64, n), make([]float64, n)
	vx, vy, vz := make([]float64, n), make([]float64, n), make([]float64, n)
	for i := 0; i < n; i++ {
		fi := float64(i)
		px[i], py[i], pz[i] = fi*0.1+1.0, fi*0.2-0.5, fi*0.05
	}
	t0 := time.Now()
	for s := 0; s < steps; s++ {
		for a := 0; a < n; a++ {
			ax, ay, az := px[a], py[a], pz[a]
			fx, fy, fz := 0.0, 0.0, 0.0
			for b := 0; b < n; b++ {
				if a != b {
					dx, dy, dz := px[b]-ax, py[b]-ay, pz[b]-az
					d2 := dx*dx + dy*dy + dz*dz + 0.01
					inv := 1.0 / (d2 * math.Sqrt(d2))
					fx += dx * inv
					fy += dy * inv
					fz += dz * inv
				}
			}
			vx[a] += fx * dt
			vy[a] += fy * dt
			vz[a] += fz * dt
		}
		for k := 0; k < n; k++ {
			px[k] += vx[k] * dt
			py[k] += vy[k] * dt
			pz[k] += vz[k] * dt
		}
	}
	ms := float64(time.Since(t0).Nanoseconds()) / 1e6
	sum := 0.0
	for i := 0; i < n; i++ {
		sum += px[i] + py[i]
	}
	return ms, int64(sum + 0.5)
}

func benchMatMul64() (float64, int64) {
	const n = 192
	const nn = n * n
	a, b, c := make([]float64, nn), make([]float64, nn), make([]float64, nn)
	for i := 0; i < nn; i++ {
		a[i] = float64(i%17) * 0.25
		b[i] = float64(i%13) * 0.5
	}
	t0 := time.Now()
	for i := 0; i < n; i++ {
		inOff := i * n
		for k := 0; k < n; k++ {
			aik := a[inOff+k]
			knOff := k * n
			for j := 0; j < n; j++ {
				c[inOff+j] += aik * b[knOff+j]
			}
		}
	}
	ms := float64(time.Since(t0).Nanoseconds()) / 1e6
	return ms, int64(c[0] + c[nn-1])
}

func benchARX() (float64, int64) {
	t0 := time.Now()
	var a, b, c, d uint32 = 0x61707865, 0x3320646E, 0x79622D32, 0x6B206574
	for i := 0; i < 2000000; i++ {
		a += b; d ^= a; d = (d << 16) | (d >> 16)
		c += d; b ^= c; b = (b << 12) | (b >> 20)
		a += b; d ^= a; d = (d << 8) | (d >> 24)
		c += d; b ^= c; b = (b << 7) | (b >> 25)
	}
	ms := float64(time.Since(t0).Nanoseconds()) / 1e6
	return ms, int64(a ^ b ^ c ^ d)
}

func benchDijkstra() (float64, int64) {
	const nv, deg = 10000, 8
	const ne = nv * deg
	const inf int64 = 1000000000000
	to, wt := make([]int, ne), make([]int64, ne)
	dist := make([]int64, nv)
	hpD, hpU := make([]int64, ne+16), make([]int, ne+16)
	var seed int64 = 1234567
	for u := 0; u < nv; u++ {
		for e := 0; e < deg; e++ {
			seed = (seed*1103515245 + 12345) & 0x7FFFFFFF
			to[u*deg+e] = (u + 1 + int(seed%97)) % nv
			wt[u*deg+e] = 1 + ((seed >> 8) % 100)
		}
		dist[u] = inf
	}
	t0 := time.Now()
	dist[0] = 0
	hsz := 1
	hpD[0], hpU[0] = 0, 0
	for hsz > 0 {
		d, cur := hpD[0], hpU[0]
		hsz--
		if hsz > 0 {
			ld, lu := hpD[hsz], hpU[hsz]
			p := 0
			for {
				l := p*2 + 1
				if l >= hsz {
					break
				}
				sm := l
				if l+1 < hsz && hpD[l+1] < hpD[l] {
					sm = l + 1
				}
				if ld <= hpD[sm] {
					break
				}
				hpD[p], hpU[p] = hpD[sm], hpU[sm]
				p = sm
			}
			hpD[p], hpU[p] = ld, lu
		}
		if d > dist[cur] {
			continue
		}
		base := cur * deg
		for e := 0; e < deg; e++ {
			nxt, nd := to[base+e], d+wt[base+e]
			if nd < dist[nxt] {
				dist[nxt] = nd
				idx := hsz
				hsz++
				for idx > 0 {
					par := (idx - 1) >> 1
					if hpD[par] <= nd {
						break
					}
					hpD[idx], hpU[idx] = hpD[par], hpU[par]
					idx = par
				}
				hpD[idx], hpU[idx] = nd, nxt
			}
		}
	}
	ms := float64(time.Since(t0).Nanoseconds()) / 1e6
	var chk int64 = 0
	for u := 0; u < nv; u++ {
		chk += dist[u]
	}
	return ms, chk
}

func benchKnapsack() (float64, int64) {
	const n, capK = 200, 15000
	dp := make([]int64, capK+1)
	wt, vl := make([]int, n), make([]int64, n)
	var seed int64 = 9876543
	for i := 0; i < n; i++ {
		seed = (seed*1103515245 + 12345) & 0x7FFFFFFF
		wt[i] = 10 + int(seed%300)
		vl[i] = 5 + ((seed >> 8) % 500)
	}
	t0 := time.Now()
	for i := 0; i < n; i++ {
		w, v := wt[i], vl[i]
		for c := capK; c >= w; c-- {
			cand := dp[c-w] + v
			if cand > dp[c] {
				dp[c] = cand
			}
		}
	}
	ms := float64(time.Since(t0).Nanoseconds()) / 1e6
	return ms, dp[capK]
}

func benchHashMap() (float64, int64) {
	const n, capH = 200000, 524288
	const mask = capH - 1
	keys, vals := make([]int64, capH), make([]int64, capH)
	t0 := time.Now()
	for i := int64(1); i <= n; i++ {
		k1 := (i * 2654435761) & 0x7FFFFFFF
		if k1 == 0 {
			k1 = 1
		}
		slot := int((uint64(k1)*11400714819323198485)>>45) & mask
		for {
			if keys[slot] == 0 || keys[slot] == k1 {
				keys[slot] = k1
				vals[slot] = i*3 + 7
				break
			}
			slot = (slot + 1) & mask
		}
	}
	var acc int64 = 0
	for i := int64(1); i <= n; i++ {
		k1 := (i * 2654435761) & 0x7FFFFFFF
		if k1 == 0 {
			k1 = 1
		}
		slot := int((uint64(k1)*11400714819323198485)>>45) & mask
		for {
			if keys[slot] == k1 {
				acc += vals[slot]
				break
			}
			if keys[slot] == 0 {
				break
			}
			slot = (slot + 1) & mask
		}
	}
	ms := float64(time.Since(t0).Nanoseconds()) / 1e6
	return ms, acc
}

func main() {
	t1, c1 := benchNBody()
	t2, c2 := benchMatMul64()
	t3, c3 := benchARX()
	t4, c4 := benchDijkstra()
	t5, c5 := benchKnapsack()
	t6, c6 := benchHashMap()
	fmt.Printf("{\"lang\": \"Go (go 1.26)\", \"nbody\": %.2f, \"matmul64\": %.2f, \"arx\": %.2f, \"dijkstra\": %.2f, \"knapsack\": %.2f, \"hashmap\": %.2f, \"chk\": [%d, %d, %d, %d, %d, %d]}\n",
		t1, t2, t3, t4, t5, t6, c1, c2, c3, c4, c5, c6)
}
