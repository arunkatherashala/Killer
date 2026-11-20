#!/usr/bin/env python3
"""Independent reference for the crypto cross-checks in tests/crypto_vectors.rs.

Standard-library only (hashlib, hmac, struct, base64). ChaCha20 and Poly1305 are written here from RFC 8439 using
Python big integers (no shared code or structure with the Rust implementation); SHA-1/256/512, HMAC and PBKDF2 come
from hashlib. Commands:

  sweep                       print one line per deterministic input: "<kind> <index> <hex>"
  kse2-decrypt B64 PASSWORD   decrypt a secure_encrypt() value, print the plain text
  kse2-encrypt PASSWORD TEXT  produce a secure_encrypt() value (salt 0..15, nonce 100..111 for repeatability)
  aead-seal KEYHEX NONCEHEX AADHEX PTHEX   print ciphertext||tag as hex
"""
import base64
import hashlib
import hmac
import struct
import sys


def poly1305(key, msg):
    r = int.from_bytes(key[:16], "little") & 0x0FFFFFFC0FFFFFFC0FFFFFFC0FFFFFFF
    s = int.from_bytes(key[16:], "little")
    p = (1 << 130) - 5
    acc = 0
    for i in range(0, len(msg), 16):
        acc = (acc + int.from_bytes(msg[i:i + 16] + b"\x01", "little")) * r % p
    return ((acc + s) & ((1 << 128) - 1)).to_bytes(16, "little")


def _rotl(x, n):
    return ((x << n) & 0xFFFFFFFF) | (x >> (32 - n))


def chacha_block(key, counter, nonce):
    st = [0x61707865, 0x3320646E, 0x79622D32, 0x6B206574] + list(struct.unpack("<8I", key)) + [counter] + list(struct.unpack("<3I", nonce))
    w = st[:]

    def qr(a, b, c, d):
        w[a] = (w[a] + w[b]) & 0xFFFFFFFF; w[d] = _rotl(w[d] ^ w[a], 16)
        w[c] = (w[c] + w[d]) & 0xFFFFFFFF; w[b] = _rotl(w[b] ^ w[c], 12)
        w[a] = (w[a] + w[b]) & 0xFFFFFFFF; w[d] = _rotl(w[d] ^ w[a], 8)
        w[c] = (w[c] + w[d]) & 0xFFFFFFFF; w[b] = _rotl(w[b] ^ w[c], 7)

    for _ in range(10):
        qr(0, 4, 8, 12); qr(1, 5, 9, 13); qr(2, 6, 10, 14); qr(3, 7, 11, 15)
        qr(0, 5, 10, 15); qr(1, 6, 11, 12); qr(2, 7, 8, 13); qr(3, 4, 9, 14)
    return struct.pack("<16I", *[(w[i] + st[i]) & 0xFFFFFFFF for i in range(16)])


def chacha_xor(key, counter, nonce, data):
    out = bytearray()
    for i in range(0, len(data), 64):
        ks = chacha_block(key, counter + i // 64, nonce)
        out += bytes(a ^ b for a, b in zip(data[i:i + 64], ks))
    return bytes(out)


def _pad16(b):
    return b"\0" * ((16 - len(b) % 16) % 16)


def aead_seal(key, nonce, aad, pt):
    otk = chacha_block(key, 0, nonce)[:32]
    ct = chacha_xor(key, 1, nonce, pt)
    mac = aad + _pad16(aad) + ct + _pad16(ct) + struct.pack("<QQ", len(aad), len(ct))
    return ct + poly1305(otk, mac)


def aead_open(key, nonce, aad, sealed):
    ct, tag = sealed[:-16], sealed[-16:]
    otk = chacha_block(key, 0, nonce)[:32]
    mac = aad + _pad16(aad) + ct + _pad16(ct) + struct.pack("<QQ", len(aad), len(ct))
    if not hmac.compare_digest(poly1305(otk, mac), tag):
        raise ValueError("authentication failed")
    return chacha_xor(key, 1, nonce, ct)


MAGIC = b"KSE2"


def kse2_encrypt(password, text, salt=bytes(range(16)), nonce=bytes(range(100, 112))):
    key = hashlib.pbkdf2_hmac("sha256", password.encode(), salt, 100000, 32)
    return base64.b64encode(MAGIC + salt + nonce + aead_seal(key, nonce, MAGIC, text.encode())).decode()


def kse2_decrypt(b64, password):
    raw = base64.b64decode(b64, validate=True)
    assert raw[:4] == MAGIC, "not a KSE2 value"
    salt, nonce, sealed = raw[4:20], raw[20:32], raw[32:]
    key = hashlib.pbkdf2_hmac("sha256", password.encode(), salt, 100000, 32)
    return aead_open(key, nonce, MAGIC, sealed).decode()


MAGIC3 = b"KSE3"


def kse3_encrypt(password, text, iterations=600000, salt=bytes(range(16)), nonce=bytes(range(100, 112))):
    header = MAGIC3 + struct.pack(">I", iterations) + salt
    key = hashlib.pbkdf2_hmac("sha256", password.encode(), salt, iterations, 32)
    return base64.b64encode(header + nonce + aead_seal(key, nonce, header, text.encode())).decode()


def kse3_decrypt(b64, password):
    raw = base64.b64decode(b64, validate=True)
    assert raw[:4] == MAGIC3, "not a KSE3 value"
    iterations = struct.unpack(">I", raw[4:8])[0]
    assert 100000 <= iterations <= 10000000, "iteration count out of range"
    header, nonce, sealed = raw[:24], raw[24:36], raw[36:]
    key = hashlib.pbkdf2_hmac("sha256", password.encode(), raw[8:24], iterations, 32)
    return aead_open(key, nonce, header, sealed).decode()


def sweep():
    """Deterministic inputs shared with the Rust test (same formulas there)."""
    for n in range(0, 300, 7):
        data = bytes((i * 31 + n) % 256 for i in range(n))
        key = data[: n % 97]
        print("sha256", n, hashlib.sha256(data).hexdigest())
        print("sha512", n, hashlib.sha512(data).hexdigest())
        print("sha1", n, hashlib.sha1(data).hexdigest())
        print("hmac", n, hmac.new(key, data, hashlib.sha256).hexdigest())
        pw, salt = data[: n % 70], data[: n % 40]
        print("pbkdf2", n, hashlib.pbkdf2_hmac("sha256", pw, salt, 1 + n % 5, 1 + n % 80).hex())
        nonce = bytes((n + i) % 256 for i in range(12))
        k32 = bytes((n * 3 + i * 5) % 256 for i in range(32))
        aad = data[: n % 29]
        print("aead", n, aead_seal(k32, nonce, aad, data).hex())


def main(argv):
    cmd = argv[1]
    if cmd == "sweep":
        sweep()
    elif cmd == "kse2-decrypt":
        sys.stdout.buffer.write(kse2_decrypt(argv[2], argv[3]).encode("utf-8"))
    elif cmd == "kse3-decrypt":
        sys.stdout.buffer.write(kse3_decrypt(argv[2], argv[3]).encode("utf-8"))
    elif cmd == "kse3-encrypt":
        sys.stdout.write(kse3_encrypt(argv[2], argv[3]))
    elif cmd == "kse2-encrypt":
        sys.stdout.write(kse2_encrypt(argv[2], argv[3]))
    elif cmd == "aead-seal":
        sys.stdout.write(aead_seal(*[bytes.fromhex(a) for a in argv[2:6]]).hex())
    else:
        raise SystemExit("unknown command " + cmd)


if __name__ == "__main__":
    main(sys.argv)
