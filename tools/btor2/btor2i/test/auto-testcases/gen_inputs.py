import random

inputs = [f"input{i}" for i in range(8)]
vals = [f'0b{"".join(str(random.randint(0, 1)) for _ in range(32))}' for i in range(8)]
print(" ".join(f"{a}={b}" for a, b in zip(inputs, vals)))
