using System;
using System.Diagnostics;

class Program
{
    static long Fibonacci(int n)
    {
        if (n <= 1) return n;
        return Fibonacci(n - 1) + Fibonacci(n - 2);
    }

    static void Main()
    {
        var sw = Stopwatch.StartNew();

        // Test 1: Loop (100,000 iterations)
        int count = 0;
        for (int i = 0; i < 100000; i++)
        {
            count++;
        }

        // Test 2: Arithmetic (50,000 operations)
        long sum = 0;
        for (int i = 0; i < 50000; i++)
        {
            sum += i;
        }

        // Test 3: Array (10,000 elements)
        var arr = new System.Collections.Generic.List<int>();
        for (int i = 0; i < 10000; i++)
        {
            arr.Add(i);
        }

        // Test 4: Recursion (fibonacci(20))
        long fibResult = Fibonacci(20);

        // Test 5: String Concatenation (1,000 times)
        string s = "";
        for (int i = 0; i < 1000; i++)
        {
            s += "a";
        }

        // Test 6: Nested Loops (100x100)
        int nestedCount = 0;
        for (int i = 0; i < 100; i++)
        {
            for (int j = 0; j < 100; j++)
            {
                nestedCount++;
            }
        }

        sw.Stop();

        Console.WriteLine("C# Benchmark Results:");
        Console.WriteLine($"Test 1: Loop count = {count}");
        Console.WriteLine($"Test 2: Arithmetic sum = {sum}");
        Console.WriteLine($"Test 3: Array size = {arr.Count}");
        Console.WriteLine($"Test 4: Fibonacci(20) = {fibResult}");
        Console.WriteLine($"Test 5: String length = {s.Length}");
        Console.WriteLine($"Test 6: Nested loops = {nestedCount}");
        Console.WriteLine($"\nTotal Execution Time: {sw.ElapsedMilliseconds} ms");
    }
}
