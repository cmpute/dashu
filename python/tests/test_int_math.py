from dashu import *


def test_roots():
    assert UBig(144).sqrt() == 12
    assert UBig(27).cbrt() == 3
    assert UBig(1024).nth_root(5) == 4
    assert UBig(12).sqr() == 144
    assert UBig(3).cubic() == 27
    # IBig: cbrt / odd nth_root are sign-preserving
    assert IBig(-27).cbrt() == -3
    assert IBig(-1024).nth_root(5) == -4
    assert IBig(144).sqrt() == 12
    # even root of negative -> ValueError, not a panic
    try:
        IBig(-4).nth_root(2)
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_number_theory():
    assert UBig(12).gcd(8) == 4
    assert UBig(12).gcd_ext(8)[0] == 4
    assert UBig(6).is_multiple_of(3)
    assert UBig(2).remove(2) == 1  # 2 = 2^1
    assert UBig(2).is_power_of_two()
    assert UBig(5).next_power_of_two() == 8


def test_bit_ops():
    n = UBig(0b10110)  # 22
    assert n.count_ones() == 3
    assert n.trailing_zeros() == 1
    assert UBig(0b1000).trailing_zeros() == 3


def test_divmod_floordiv():
    assert UBig(17) // 5 == 3
    q, r = divmod(UBig(17), 5)
    assert q == 3 and r == 2
    assert UBig(20) // 3 == 6
    assert IBig(-17) // 5 == -4


def test_floordiv_mod_floored_semantics():
    # CPython floor division: the quotient rounds toward -inf and the remainder carries the
    # sign of the divisor (differs from both truncating and Euclidean division for negatives).
    assert IBig(-7) % 3 == 2
    assert IBig(7) % -3 == -2
    assert IBig(-7) % -3 == -1
    assert IBig(7) // -3 == -3
    assert IBig(-7) // 3 == -3
    assert IBig(7) // 3 == 2
    # divmod is consistent with // and %, and satisfies a == b*(a//b) + (a%b)
    for a in (IBig(-7), IBig(7), IBig(-8), IBig(9), UBig(7)):
        for b in (3, -3, 5, -5):
            q, r = divmod(a, b)
            assert q == a // b
            assert r == a % b
            assert q * b + r == a


def test_pow_with_modulus():
    assert pow(IBig(2), 3, 5) == 3     # 8 mod 5
    assert pow(IBig(2), 3, -5) == -2   # CPython: result carries the modulus sign
    assert pow(IBig(5), 2, -5) == 0    # exact multiple -> 0 regardless of sign


def test_inplace_ops():
    n = UBig(10)
    n += 5
    assert n == 15
    n -= 3
    assert n == 12
    n *= 2
    assert n == 24
    n <<= 1
    assert n == 48
    n >>= 2
    assert n == 12
    n &= 4  # 12 & 4
    assert n == 4


def test_chunks_words():
    n = UBig("0x123456789abcdef")
    assert UBig.from_chunks(n.to_chunks(10), 10) == n
    w = n.to_words()
    assert UBig.from_words(w) == n


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("int math tests passed")
