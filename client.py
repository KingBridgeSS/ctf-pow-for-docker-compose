import hashlib
import string
import itertools

from pwn import remote
import re


def proof_of_work(
    nonce: str,
    difficulty: int,
    salt_charset: str = string.ascii_letters + string.digits,
):
    nonce_byte = nonce.encode()
    expected_prefix = "0" * difficulty
    for salt in itertools.chain.from_iterable(
        map(
            bytes,
            itertools.product(salt_charset.encode(), repeat=i),
        )
        for i in itertools.count(1)
    ):
        if hashlib.sha256(nonce_byte + salt).hexdigest().startswith(expected_prefix):
            return salt
    raise ValueError("No solution found")


p = remote("localhost", 1337)
rev= p.recvuntil(b"== ").decode()
print(rev)
matched = re.search(
    r"sha256\('(?P<nonce>[-A-Za-z0-9+/]+?)'.*?\)(?:.*?)startswith\('0'\s*\*\s*(?P<diff>\d+)\)",
    rev,
)
assert matched, "No proof of work found"
nonce = matched.group("nonce")
difficulty = int(matched.group("diff"))

p.send(proof_of_work(nonce, difficulty))
p.interactive()
