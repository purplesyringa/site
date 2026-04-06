import struct
from pathlib import Path
import random

def get_paragraph_dataset() -> list[str]:
    paragraphs = set()
    for child in Path("..").iterdir():
        try:
            with (child / "index.md").open() as f:
                for paragraph in f.read().split("\n\n"):
                    paragraph = paragraph.strip()
                    if paragraph:
                        paragraphs.add(paragraph)
        except (FileNotFoundError, NotADirectoryError):
            pass
    return list(sorted(paragraphs))

def get_domain_dataset() -> list[str]:
    with open("domains.txt") as f:
        return [domain.strip() for domain in f]

strings = get_domain_dataset()
print(len(strings))
print(sum(map(len, strings)) / len(strings))
print(random.choice(strings))

def my_hash(s: str) -> int:
    # import hashlib
    # return struct.unpack("<I", hashlib.sha256(s.encode()).digest()[:4])[0]
    s = s.encode()
    s += b"\x00" * (-len(s) % 4)
    words = struct.iter_unpack("<I", s)

    h = 0
    for word, in words:
        # h = ((h << 3) % 2**32) | (h >> 29)
        h = (h + word) % 2**32

    prod = h * 0x2e4a7ba5
    return (prod % 2**32) ^ (prod >> 32)

stats = [0] * 32
correlation = [[0] * 32 for bit1 in range(32)]
for string in strings:
    h = my_hash(string)
    for bit1 in range(32):
        stats[bit1] += (h >> bit1) & 1
        for bit2 in range(bit1):
            correlation[bit1][bit2] += ((h >> bit1) & 1) == ((h >> bit2) & 1)
for bit1 in range(32):
    for bit2 in range(bit1 + 1, 32):
        correlation[bit1][bit2] = correlation[bit2][bit1]

for bit in range(32):
    cell = str(round(stats[bit] / len(strings) * 100) - 50) + "%"
    print(cell, end="\t")
print()
print()

for bit1 in range(32):
    for bit2 in range(32):
        if bit1 == bit2:
            cell = ""
        else:
            cell = str(round(correlation[bit1][bit2] / len(strings) * 200) - 100)
        # print(cell.rjust(3), end=" ")
        print(cell, end="\t")
    print()

hash_table = [0] * 4096
collisions = 0
for string in strings:
    bucket = my_hash(string) & 4095
    collisions += hash_table[bucket]
    hash_table[bucket] += 1
print("collisions:", collisions)

print(len({my_hash(string) for string in strings}))
