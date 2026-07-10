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
