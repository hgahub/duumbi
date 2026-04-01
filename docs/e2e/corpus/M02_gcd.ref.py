"""Reference implementation: Find the greatest common divisor of two numbers"""

def gcd(a, b):
    a, b = abs(a), abs(b)
    while b:
        a, b = b, a % b
    return a

print("=== Greatest Common Divisor ===")
print(f"gcd(48, 18) = {gcd(48, 18)}")
print(f"gcd(17, 5) = {gcd(17, 5)}")
print(f"gcd(7, 7) = {gcd(7, 7)}")
print(f"gcd(15, 0) = {gcd(15, 0)}")
print(f"gcd(-12, 8) = {gcd(-12, 8)}")
