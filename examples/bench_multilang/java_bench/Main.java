package java_bench;

import java.util.Arrays;

public class Main {
    // --- W1: Matrix GEMM 256x256 FP32 ---
    static double benchGEMM() {
        int n = 256;
        float[] a = new float[n * n];
        float[] b = new float[n * n];
        float[] c = new float[n * n];
        Arrays.fill(a, 0.5f);
        Arrays.fill(b, 0.5f);

        long t0 = System.nanoTime();
        for (int i = 0; i < n; i++) {
            for (int k = 0; k < n; k++) {
                float aik = a[i * n + k];
                for (int j = 0; j < n; j++) {
                    c[i * n + j] += aik * b[k * n + j];
                }
            }
        }
        double elapsed = (System.nanoTime() - t0) / 1_000_000.0;
        if (c[0] < 0) System.out.println("checksum: " + c[0]);
        return elapsed;
    }

    // --- W2: BNN 1-Bit POPCOUNT GEMV (4096 rows x 4096 cols = 64 words) ---
    static double benchBNN() {
        int rows = 4096;
        int words = 64;
        long[] weights = new long[rows * words];
        long[] act = new long[words];
        int[] out = new int[rows];

        Arrays.fill(act, 0xAAAAAAAAAAAAAAAAL);
        Arrays.fill(weights, 0x5555555555555555L);

        long t0 = System.nanoTime();
        for (int r = 0; r < rows; r++) {
            int sum = 0;
            int base = r * words;
            for (int w = 0; w < words; w++) {
                long x = weights[base + w] ^ act[w];
                sum += Long.bitCount(x);
            }
            out[r] = sum;
        }
        double elapsed = (System.nanoTime() - t0) / 1_000_000.0;
        if (out[0] < 0) System.out.println("checksum: " + out[0]);
        return elapsed;
    }

    // --- W3: Quicksort (100,000 int64) ---
    static void quicksortI64(long[] arr, int low, int high) {
        if (low >= high) return;
        long pivot = arr[high];
        int i = low - 1;
        for (int j = low; j < high; j++) {
            if (arr[j] <= pivot) {
                i++;
                long tmp = arr[i];
                arr[i] = arr[j];
                arr[j] = tmp;
            }
        }
        long tmp = arr[i + 1];
        arr[i + 1] = arr[high];
        arr[high] = tmp;
        int pi = i + 1;

        quicksortI64(arr, low, pi - 1);
        quicksortI64(arr, pi + 1, high);
    }

    static double benchSort() {
        int n = 100000;
        long[] arr = new long[n];
        long seed = 42;
        for (int i = 0; i < n; i++) {
            seed = (seed * 1103515245L + 12345L) & 0x7FFFFFFFL;
            arr[i] = seed % 1000000L;
        }

        long t0 = System.nanoTime();
        quicksortI64(arr, 0, n - 1);
        double elapsed = (System.nanoTime() - t0) / 1_000_000.0;

        if (arr[0] < 0) System.out.println("checksum: " + arr[0]);
        return elapsed;
    }

    // --- W4: Sieve of Eratosthenes (2,000,000) ---
    static double benchSieve() {
        int n = 2000000;
        boolean[] isPrime = new boolean[n + 1];
        Arrays.fill(isPrime, true);
        isPrime[0] = false;
        isPrime[1] = false;

        long t0 = System.nanoTime();
        for (int p = 2; p * p <= n; p++) {
            if (isPrime[p]) {
                for (int i = p * p; i <= n; i += p) {
                    isPrime[i] = false;
                }
            }
        }

        int count = 0;
        for (int i = 2; i <= n; i++) {
            if (isPrime[i]) count++;
        }
        double elapsed = (System.nanoTime() - t0) / 1_000_000.0;

        if (count == 0) System.out.println("checksum: " + count);
        return elapsed;
    }

    // --- W5: 3D Particle Kinematics (50,000 particles x 10 steps) ---
    static double benchPhysics() {
        int n = 50000;
        float[] x = new float[n];
        float[] y = new float[n];
        float[] z = new float[n];
        float[] vx = new float[n];
        float[] vy = new float[n];
        float[] vz = new float[n];

        Arrays.fill(y, 100.0f);
        Arrays.fill(vx, 1.0f);
        Arrays.fill(vz, 0.5f);

        float dt = 0.016f;
        float g = -9.81f;

        long t0 = System.nanoTime();
        for (int step = 0; step < 10; step++) {
            for (int i = 0; i < n; i++) {
                x[i] += vx[i] * dt;
                y[i] += vy[i] * dt;
                z[i] += vz[i] * dt;
                vy[i] += g * dt;
            }
        }
        double elapsed = (System.nanoTime() - t0) / 1_000_000.0;

        if (y[0] < -9999.0f) System.out.println("checksum: " + y[0]);
        return elapsed;
    }

    // --- W6: JSON Scanner (100,000 runs) ---
    static double benchJSON() {
        String doc = "{\"order_id\": 9876543, \"user_id\": 42109, \"symbol\": \"BTCUSD\", \"qty\": 1500, \"price\": 96500, \"status\": \"FILLED\"}";
        byte[] docBytes = doc.getBytes();
        int docLen = docBytes.length;
        byte[] key = "\"price\": ".getBytes();
        int keyLen = key.length;

        long t0 = System.nanoTime();
        long sumPrice = 0;
        for (int r = 0; r < 100000; r++) {
            for (int i = 0; i <= docLen - keyLen; i++) {
                if (docBytes[i] == '"' && docBytes[i + 1] == 'p') {
                    boolean match = true;
                    for (int k = 0; k < keyLen; k++) {
                        if (docBytes[i + k] != key[k]) {
                            match = false;
                            break;
                        }
                    }
                    if (match) {
                        long val = 0;
                        int k = i + keyLen;
                        while (k < docLen && docBytes[k] >= '0' && docBytes[k] <= '9') {
                            val = val * 10 + (docBytes[k] - '0');
                            k++;
                        }
                        sumPrice += val;
                        break;
                    }
                }
            }
        }
        double elapsed = (System.nanoTime() - t0) / 1_000_000.0;

        if (sumPrice == 0) System.out.println("checksum: " + sumPrice);
        return elapsed;
    }

    static long fibJava(int n) {
        return n <= 1 ? n : fibJava(n - 1) + fibJava(n - 2);
    }

    static double benchFib() {
        long t0 = System.nanoTime();
        long r = fibJava(35);
        double el = (System.nanoTime() - t0) / 1_000_000.0;
        if (r != 9227465) System.out.println("checksum: " + r);
        return el;
    }

    static double benchCRC32() {
        int n = 16777216;
        byte[] buf = new byte[n];
        int[] tbl = new int[256];
        for (int i = 0; i < 256; i++) {
            int c = i;
            for (int k = 0; k < 8; k++) c = (c & 1) != 0 ? ((c >>> 1) ^ 0xEDB88320) : (c >>> 1);
            tbl[i] = c;
        }
        for (int i = 0; i < n; i++) buf[i] = (byte)((i * 7 + 13) & 255);
        long t0 = System.nanoTime();
        int crc = 0xFFFFFFFF;
        for (int i = 0; i < n; i++) crc = (crc >>> 8) ^ tbl[(crc ^ buf[i]) & 255];
        crc ^= 0xFFFFFFFF;
        double el = (System.nanoTime() - t0) / 1_000_000.0;
        if (crc == 0) System.out.println("checksum: " + crc);
        return el;
    }

    static double benchMandel() {
        int size = 512;
        long t0 = System.nanoTime();
        long total = 0;
        for (int py = 0; py < size; py++) {
            double ci = (py / 512.0) * 2.4 - 1.2;
            for (int px = 0; px < size; px++) {
                double cr = (px / 512.0) * 3.0 - 2.0;
                double zr = 0.0, zi = 0.0;
                int it = 0;
                while (it < 64) {
                    double zr2 = zr * zr, zi2 = zi * zi;
                    if (zr2 + zi2 > 4.0) break;
                    zi = 2.0 * zr * zi + ci;
                    zr = zr2 - zi2 + cr;
                    it++;
                }
                total += it;
            }
        }
        double el = (System.nanoTime() - t0) / 1_000_000.0;
        if (total == 0) System.out.println("checksum: " + total);
        return el;
    }

    static double benchGCChurn() {
        long t0 = System.nanoTime();
        long acc = 0;
        for (int i = 0; i < 1000000; i++) {
            long[] b = new long[8];
            b[0] = i & 255;
            acc += b[0];
        }
        double el = (System.nanoTime() - t0) / 1_000_000.0;
        if (acc == 0) System.out.println("checksum: " + acc);
        return el;
    }

    public static void main(String[] args) {
        double tGemm = benchGEMM();
        double tBnn = benchBNN();
        double tSort = benchSort();
        double tSieve = benchSieve();
        double tPhys = benchPhysics();
        double tJson = benchJSON();
        double tFib = benchFib();
        double tCrc = benchCRC32();
        double tMandel = benchMandel();
        double tGc = benchGCChurn();

        System.out.printf("{\"lang\": \"Java (JDK 21)\", \"gemm\": %.2f, \"bnn\": %.2f, \"sort\": %.2f, \"sieve\": %.2f, \"physics\": %.2f, \"json\": %.2f, \"fib\": %.2f, \"crc32\": %.2f, \"mandel\": %.2f, \"gc_churn\": %.2f}\n",
                tGemm, tBnn, tSort, tSieve, tPhys, tJson, tFib, tCrc, tMandel, tGc);
    }
}
