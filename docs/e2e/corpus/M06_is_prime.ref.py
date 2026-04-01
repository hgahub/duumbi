"""Reference implementation: Check if a number is prime"""

def is_prime(n):
    if n <= 1:
        return 0
    if n <= 3:
        return 1
    if n % 2 == 0:
        return 0
    d = 3
    while d * d <= n:
        if n % d == 0:
            return 0
        d += 2
    return 1

print("=== Prime Checker ===")
print(f"is_prime(2) = {is_prime(2)}")
print(f"is_prime(17) = {is_prime(17)}")
print(f"is_prime(4) = {is_prime(4)}")
print(f"is_prime(1) = {is_prime(1)}")
print(f"is_prime(0) = {is_prime(0)}")
print(f"is_prime(-7) = {is_prime(-7)}")
