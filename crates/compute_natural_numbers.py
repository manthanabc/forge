#!/usr/bin/env python3
"""
Script to compute and display the first 8 natural numbers
"""

def compute_first_n_natural_numbers(n):
    """
    Compute the first n natural numbers
    Natural numbers start from 1: 1, 2, 3, 4, ...
    """
    return list(range(1, n + 1))

def main():
    # Compute first 8 natural numbers
    n = 8
    natural_numbers = compute_first_n_natural_numbers(n)
    
    print(f"The first {n} natural numbers are:")
    print(natural_numbers)
    
    # Also display them in a formatted way
    print("\nFormatted output:")
    for i, num in enumerate(natural_numbers, 1):
        print(f"{i}: {num}")
    
    # Show sum as bonus
    total = sum(natural_numbers)
    print(f"\nSum of first {n} natural numbers: {total}")

if __name__ == "__main__":
    main()