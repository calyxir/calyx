import random
import os
import subprocess


def generate_binary_string(length):
    """Generate a random binary string of given length."""
    return "".join(random.choice("01") for _ in range(length))


def generate_tests(num_tests):
    """Generate `num_tests` random tests."""
    tests = []
    for _ in range(num_tests):
        # Generate a random binary string (up to 32 bits for u32 in Rust)
        binary_string = generate_binary_string(random.randint(1, 24))
        tests.append((binary_string))

    return tests


def write_input_files(tests, input_dir):
    os.makedirs(input_dir, exist_ok=True)
    input_paths = []

    for binary_string in tests:
        input_path = os.path.join(input_dir, f"speed_test.in")
        with open(input_path, "a") as f:
            f.write(f"{binary_string}\n")
        input_paths.append(input_path)

    return input_paths


if __name__ == "__main__":
    num_tests = 1_000_000
    input_dir = "./testsuite/inputs"
    exponent = -4

    # Generate Tests
    tests = generate_tests(num_tests)

    # Write Input File
    write_input_files(tests, input_dir)

    print(f"Input files are located in {input_dir}.")
