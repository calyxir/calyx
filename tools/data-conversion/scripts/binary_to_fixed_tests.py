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

    for idx, binary_string in enumerate(tests):
        input_path = os.path.join(input_dir, f"test_{idx+1}.in")
        with open(input_path, "w") as f:
            f.write(f"{binary_string}")
        input_paths.append(input_path)

    return input_paths


def write_expect_files(tests, expect_dir):
    os.makedirs(expect_dir, exist_ok=True)
    expect_paths = []

    for idx, binary_string in enumerate(tests):
        expect_path = os.path.join(expect_dir, f"test_{idx+1}.expect")
        with open(expect_path, "w") as f:
            f.write(f"{binary_string}")
        expect_paths.append(expect_path)


def convert_binary_to_fixed(binary_string, exponent):
    """Convert binary string to a fixed-point number."""
    binary_value = int(binary_string, 2)  # Convert binary string to integer
    fixed_point_number = binary_value * (
        2 ** exponent
    )  # Calculate the fixed-point number
    formatted = "{:.8e}".format(fixed_point_number)
    return formatted + "\n"


def run_rust_function(input_file, output_file, exponent):
    rust_command = f"../../target/debug/data-conversion --from {input_file} --to {output_file} --ftype 'binary' --totype 'fixed' --exp {exponent}"
    result = subprocess.run(rust_command, shell=True, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Can't run rust function")
    return result.returncode == 0


def compare_files(output_file, expect_file):
    with open(output_file, "r") as f:
        output_content = f.read().strip()
        output_content = float(output_content)
    with open(expect_file, "r") as f:
        expect_content = f.read().strip()
        expect_content = float(expect_content)

    return output_content == expect_content


if __name__ == "__main__":
    num_tests = 100
    input_dir = "../testsuite/inputs"
    expect_dir = "../testsuite/expect"
    output_dir = "../testsuite/outputs"
    exponent = -4

    # Generate Tests
    tests = generate_tests(num_tests)

    # Write Input Files
    input_paths = write_input_files(tests, input_dir)

    # Generate Expected Output
    results = []
    for binary_string in tests:
        results.append(convert_binary_to_fixed(binary_string, exponent))
    expect_paths = write_expect_files(results, expect_dir)

    # Make sure the output directory exists
    os.makedirs(output_dir, exist_ok=True)

    # Run Tests and Compare Outputs
    for idx, test_file in enumerate(input_paths):
        input_file = test_file
        output_file = os.path.join(output_dir, f"test_{idx+1}.out")
        expect_file = os.path.join(expect_dir, f"test_{idx+1}.expect")

        if run_rust_function(input_file, output_file, exponent):
            if compare_files(output_file, expect_file):
                print(f"Test {idx+1} passed.")
            else:
                print(f"Test {idx+1} failed: output does not match expected.")
        else:
            print(f"Test {idx+1} failed to run.")

    print(f"Input files are located in {input_dir}/.")
    print(f"Expected output files are located in {expect_dir}/.")
    print(f"Output files are located in {output_dir}/.")
