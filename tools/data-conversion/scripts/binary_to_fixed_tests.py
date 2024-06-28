import random
import yaml
import os

def generate_binary_string(length):
    """Generate a random binary string of given length."""
    return ''.join(random.choice('01') for _ in range(length))

def generate_tests(num_tests):
    """Generate `num_tests` random tests."""
    tests = []
    for _ in range(num_tests):
        # Generate a random binary string (up to 32 bits for u32 in Rust)
        binary_string = generate_binary_string(random.randint(1, 32))
        tests.append((binary_string))
    
    return tests

def write_input_files(tests, input_dir):
    os.makedirs(input_dir, exist_ok=True)
    input_paths = []

    for idx, binary_string in enumerate(tests):
        input_path = os.path.join(input_dir, f"test_{idx+1}.in")
        with open(input_path, 'w') as f:
            f.write(f"{binary_string}")
        input_paths.append(input_path)

    return input_paths

def write_expect_files(tests, expect_dir):
    os.makedirs(expect_dir, exist_ok=True)
    expect_paths = []

    for idx, binary_string in enumerate(tests):
        expect_path = os.path.join(expect_dir, f"test_{idx+1}.expect")
        with open(expect_path, 'w') as f:
            f.write(f"{binary_string}")
        expect_paths.append(expect_path)

def convert_binary_to_fixed(binary_string, exponent):
    """Convert binary string to a fixed-point number."""
    binary_value = int(binary_string, 2)  # Convert binary string to integer
    fixed_point_number = binary_value * (2 ** exponent)  # Calculate the fixed-point number
    return str(fixed_point_number) + "\n"

if __name__ == '__main__':
    num_tests = 10
    input_dir = "/Users/Angelica/Desktop/calyx/tools/data-conversion/testsuite/inputs"
    expect_dir = "/Users/Angelica/Desktop/calyx/tools/data-conversion/testsuite/expect"
    yaml_filename = "/Users/Angelica/Desktop/calyx/tools/data-conversion/testsuite/testsuite.yaml"
    
    tests = generate_tests(num_tests)
    results = []
    for i in range(len(tests)):
        results.append(convert_binary_to_fixed(tests[i], -4))

    input_paths = write_input_files(tests, input_dir)
    expect_paths = write_expect_files(results, expect_dir)
    
    print(f"Input files are located in {input_dir}/.")
