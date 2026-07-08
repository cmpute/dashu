from dashu import *


def test_roots():
    assert UBig(144).sqrt() == UBig(12)
    assert UBig(27).cbrt() == UBig(3)
    assert UBig(1024).nth_root(5) == UBig(4)
    assert UBig(12).sqr() == UBig(144)
    assert UBig(3).cubic() == UBig(27)
    # IBig: cbrt / odd nth_root are sign-preserving
    assert IBig(-27).cbrt() == IBig(-3)
    assert IBig(-1024).nth_root(5) == IBig(-4)
    assert IBig(144).sqrt() == UBig(12)
    # even root of negative -> ValueError, not a panic
    try:
        IBig(-4).nth_root(2)
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_number_theory():
    assert UBig(12).gcd(UBig(8)) == UBig(4)
    g, x, y = UBig(12).gcd_ext(UBig(8))
    assert g == UBig(4)
    assert UBig(12) * x + UBig(8) * y == g
    assert UBig(6).is_multiple_of(UBig(3))
    assert UBig(2).remove(UBig(2)) == 1  # 2 = 2^1
    assert UBig(2).is_power_of_two()
    assert UBig(5).next_power_of_two() == UBig(8)


def test_bit_ops():
    n = UBig(0b10110)  # 22
    assert n.count_ones() == 3
    assert n.trailing_zeros() == 1
    assert n.trailing_ones() == 0
    assert UBig(0b1000).trailing_zeros() == 3


def test_divmod_floordiv():
    assert int(UBig(17) // UBig(5)) == 3
    q, r = divmod(UBig(17), UBig(5))
    assert int(q) == 3 and int(r) == 2
    assert int(UBig(20) // 3) == 6
    # IBig floor division
    assert int(IBig(-17) // IBig(5)) == -4


def test_inplace_ops():
    n = UBig(10)
    n += UBig(5)
    assert int(n) == 15
    n -= UBig(3)
    assert int(n) == 12
    n *= UBig(2)
    assert int(n) == 24
    n <<= 1
    assert int(n) == 48
    n >>= 2
    assert int(n) == 12
    n &= UBig(0b100)  # 12 & 4
    assert int(n) == 4


def test_chunks_words():
    n = UBig("0x123456789abcdef")
    assert UBig.from_chunks(n.to_chunks(10), 10) == n
    w = n.to_words()
    assert UBig.from_words(w) == n


if __name__ == "__main__":
    test_roots()
    test_number_theory()
    test_bit_ops()
    test_divmod_floordiv()
    test_inplace_ops()
    test_chunks_words()
    print("int math tests passed")
