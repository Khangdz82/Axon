using System;
using System.Diagnostics;
using System.Numerics;
using System.Text;

namespace CSBench;

class Program
{
    // --- W1: Matrix GEMM 256x256 FP32 ---
    static double BenchGEMM()
    {
        int n = 256;
        float[] a = new float[n * n];
        float[] b = new float[n * n];
        float[] c = new float[n * n];
        Array.Fill(a, 0.5f);
        Array.Fill(b, 0.5f);

        var sw = Stopwatch.StartNew();
        for (int i = 0; i < n; i++)
        {
            for (int k = 0; k < n; k++)
            {
                float aik = a[i * n + k];
                for (int j = 0; j < n; j++)
                {
                    c[i * n + j] += aik * b[k * n + j];
                }
            }
        }
        sw.Stop();
        if (c[0] < 0) Console.WriteLine("checksum: " + c[0]);
        return sw.Elapsed.TotalMilliseconds;
    }

    // --- W2: BNN 1-Bit POPCOUNT GEMV (4096 rows x 4096 cols = 64 words) ---
    static double BenchBNN()
    {
        int rows = 4096;
        int words = 64;
        ulong[] weights = new ulong[rows * words];
        ulong[] act = new ulong[words];
        int[] output = new int[rows];

        Array.Fill(act, 0xAAAAAAAAAAAAAAAAUL);
        Array.Fill(weights, 0x5555555555555555UL);

        var sw = Stopwatch.StartNew();
        for (int r = 0; r < rows; r++)
        {
            int sum = 0;
            int bBase = r * words;
            for (int w = 0; w < words; w++)
            {
                ulong x = weights[bBase + w] ^ act[w];
                sum += BitOperations.PopCount(x);
            }
            output[r] = sum;
        }
        sw.Stop();
        if (output[0] < 0) Console.WriteLine("checksum: " + output[0]);
        return sw.Elapsed.TotalMilliseconds;
    }

    // --- W3: Quicksort (100,000 int64) ---
    static void QuicksortI64(long[] arr, int low, int high)
    {
        if (low >= high) return;
        long pivot = arr[high];
        int i = low - 1;
        for (int j = low; j < high; j++)
        {
            if (arr[j] <= pivot)
            {
                i++;
                long tmp = arr[i];
                arr[i] = arr[j];
                arr[j] = tmp;
            }
        }
        long tmp2 = arr[i + 1];
        arr[i + 1] = arr[high];
        arr[high] = tmp2;
        int pi = i + 1;

        QuicksortI64(arr, low, pi - 1);
        QuicksortI64(arr, pi + 1, high);
    }

    static double BenchSort()
    {
        int n = 100000;
        long[] arr = new long[n];
        uint seed = 42;
        for (int i = 0; i < n; i++)
        {
            seed = (seed * 1103515245U + 12345U) & 0x7FFFFFFFU;
            arr[i] = (long)(seed % 1000000U);
        }

        var sw = Stopwatch.StartNew();
        QuicksortI64(arr, 0, n - 1);
        sw.Stop();

        if (arr[0] < 0) Console.WriteLine("checksum: " + arr[0]);
        return sw.Elapsed.TotalMilliseconds;
    }

    // --- W4: Sieve of Eratosthenes (2,000,000) ---
    static double BenchSieve()
    {
        int n = 2000000;
        byte[] isPrime = new byte[n + 1];
        Array.Fill(isPrime, (byte)1);
        isPrime[0] = 0;
        isPrime[1] = 0;

        var sw = Stopwatch.StartNew();
        for (int p = 2; p * p <= n; p++)
        {
            if (isPrime[p] == 1)
            {
                for (int i = p * p; i <= n; i += p)
                {
                    isPrime[i] = 0;
                }
            }
        }

        int count = 0;
        for (int i = 2; i <= n; i++)
        {
            if (isPrime[i] == 1) count++;
        }
        sw.Stop();

        if (count == 0) Console.WriteLine("checksum: " + count);
        return sw.Elapsed.TotalMilliseconds;
    }

    // --- W5: 3D Particle Kinematics (50,000 particles x 10 steps) ---
    struct Particle
    {
        public float X, Y, Z;
        public float Vx, Vy, Vz;
    }

    static double BenchPhysics()
    {
        int n = 50000;
        Particle[] p = new Particle[n];
        for (int i = 0; i < n; i++)
        {
            p[i].X = 0.0f; p[i].Y = 100.0f; p[i].Z = 0.0f;
            p[i].Vx = 1.0f; p[i].Vy = 0.0f; p[i].Vz = 0.5f;
        }
        float dt = 0.016f;
        float g = -9.81f;

        var sw = Stopwatch.StartNew();
        for (int step = 0; step < 10; step++)
        {
            for (int i = 0; i < n; i++)
            {
                p[i].X += p[i].Vx * dt;
                p[i].Y += p[i].Vy * dt;
                p[i].Z += p[i].Vz * dt;
                p[i].Vy += g * dt;
            }
        }
        sw.Stop();

        if (p[0].Y < -9999.0f) Console.WriteLine("checksum: " + p[0].Y);
        return sw.Elapsed.TotalMilliseconds;
    }

    // --- W6: JSON Scanner (100,000 runs) ---
    static double BenchJSON()
    {
        string doc = "{\"order_id\": 9876543, \"user_id\": 42109, \"symbol\": \"BTCUSD\", \"qty\": 1500, \"price\": 96500, \"status\": \"FILLED\"}";
        byte[] docBytes = Encoding.UTF8.GetBytes(doc);
        int docLen = docBytes.Length;
        byte[] key = Encoding.UTF8.GetBytes("\"price\": ");
        int keyLen = key.Length;

        var sw = Stopwatch.StartNew();
        long sumPrice = 0;
        for (int r = 0; r < 100000; r++)
        {
            for (int i = 0; i <= docLen - keyLen; i++)
            {
                if (docBytes[i] == (byte)'\"' && docBytes[i + 1] == (byte)'p')
                {
                    bool match = true;
                    for (int k = 0; k < keyLen; k++)
                    {
                        if (docBytes[i + k] != key[k])
                        {
                            match = false;
                            break;
                        }
                    }
                    if (match)
                    {
                        long val = 0;
                        int k = i + keyLen;
                        while (k < docLen && docBytes[k] >= (byte)'0' && docBytes[k] <= (byte)'9')
                        {
                            val = val * 10 + (docBytes[k] - (byte)'0');
                            k++;
                        }
                        sumPrice += val;
                        break;
                    }
                }
            }
        }
        sw.Stop();

        if (sumPrice == 0) Console.WriteLine("checksum: " + sumPrice);
        return sw.Elapsed.TotalMilliseconds;
    }

    static void Main()
    {
        double tGemm = BenchGEMM();
        double tBnn = BenchBNN();
        double tSort = BenchSort();
        double tSieve = BenchSieve();
        double tPhys = BenchPhysics();
        double tJson = BenchJSON();

        Console.WriteLine($"{{\"lang\": \"C# (.NET 10)\", \"gemm\": {tGemm:F2}, \"bnn\": {tBnn:F2}, \"sort\": {tSort:F2}, \"sieve\": {tSieve:F2}, \"physics\": {tPhys:F2}, \"json\": {tJson:F2}}}");
    }
}
