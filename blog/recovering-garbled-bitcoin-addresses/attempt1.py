import base58
import itertools


address_lowercase = "1lbcfr7sahtd9cgdqo3htmtkv8lk4znx71"


def try_both_cases(c):
    yield c
    if c.upper() != c:
        yield c.upper()

for address in itertools.product(*map(try_both_cases, address_lowercase)):
    address = "".join(address)
    try:
        base58.b58decode_check(address)
    except ValueError:
        pass
    else:
        print("Found valid address:", address)
