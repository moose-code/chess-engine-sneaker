#!/usr/bin/env python3
import random
from pathlib import Path

# Create a small random NNUE-style model file for experimentation.
# Real strength requires trained weights.

OUT = Path("benchmarks/weights/nnue-random.txt")
OUT.parent.mkdir(parents=True, exist_ok=True)
H = 32
random.seed(13)

vals = [H]
vals += [0 for _ in range(H)]  # hidden bias
vals += [random.randint(-8, 8) for _ in range(H)]  # output weights
vals += [0]  # output bias
vals += [random.randint(-2, 2) for _ in range(768 * H)]

OUT.write_text(" ".join(str(x) for x in vals))
print(f"wrote {OUT}")
