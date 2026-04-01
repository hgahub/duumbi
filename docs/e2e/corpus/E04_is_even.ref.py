"""Reference implementation: Check if a number is even"""

def is_even(n):
    return 1 if n % 2 == 0 else 0

print("=== Even Number Checker ===")
print(f"is_even(4) = {is_even(4)}")
print(f"is_even(7) = {is_even(7)}")
print(f"is_even(0) = {is_even(0)}")
print(f"is_even(-6) = {is_even(-6)}")
print(f"is_even(-3) = {is_even(-3)}")
