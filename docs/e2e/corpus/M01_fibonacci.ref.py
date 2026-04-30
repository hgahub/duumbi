"""Reference implementation: Write a fibonacci function"""

def fib(n):
    if n < 0:
        return 0
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

print("=== Fibonacci ===")
print(f"fib(0) = {fib(0)}")
print(f"fib(1) = {fib(1)}")
print(f"fib(2) = {fib(2)}")
print(f"fib(6) = {fib(6)}")
print(f"fib(10) = {fib(10)}")
