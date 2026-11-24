from decimal import Decimal
from fractions import Fraction

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


def test_ubig_subtraction_below_zero():
    # the difference of two UBig (or a UBig and a non-negative int) is an IBig when it is
    # negative, instead of a Rust panic
    for result in (UBig(0) - 1, UBig(0) - UBig(1), 0 - UBig(1), UBig(3) - 10, 3 - UBig(10)):
        assert isinstance(result, IBig)
    assert UBig(0) - 1 == -1 and 0 - UBig(1) == -1
    assert UBig(3) - UBig(10) == -7 and 3 - UBig(10) == -7
    assert UBig(2**70) - UBig(2**71) == -(2**70)
    # a non-negative difference stays a UBig
    for result in (UBig(10) - 3, UBig(10) - UBig(10), 10 - UBig(3)):
        assert isinstance(result, UBig)
    assert UBig(10) - UBig(10) == 0 and 10 - UBig(3) == 7
    assert UBig(1) - (-2) == 3

    n = UBig(0)
    n -= 1
    assert isinstance(n, IBig) and n == -1
    n = UBig(5)
    n -= 2
    assert isinstance(n, UBig) and n == 3


def test_inplace_ops_agree_with_binary_ops():
    # `m op= b` gives the same value as `m = m op b`, also when the operand (or the result)
    # is not an unsigned integer, where the in-place form used to raise TypeError
    import operator
    ops = [
        ("+", operator.add), ("-", operator.sub), ("*", operator.mul),
        ("&", operator.and_), ("|", operator.or_), ("^", operator.xor),
    ]
    operands = [3, 0, -3, -1, -4, 2**70, -(2**70), UBig(3), IBig(-3)]
    for cls, start in ((UBig, 10), (UBig, 0), (UBig, 12), (IBig, 10), (IBig, -10)):
        for symbol, op in ops:
            for b in operands:
                scope = {"m": cls(start), "b": b}
                exec(f"m {symbol}= b", {}, scope)
                expected = op(start, int(b))
                assert int(scope["m"]) == expected, (cls.__name__, start, symbol, b)
                # a UBig only stays one while its value is not negative
                assert isinstance(scope["m"], IBig if expected < 0 else (UBig, IBig))

    m = UBig(10)
    m += -3
    assert isinstance(m, IBig) and m == 7
    m = UBig(12)
    m &= -4
    assert isinstance(m, UBig) and m == 12

    # non-integer operands fall back to the binary operators as well
    m = IBig(1)
    m += 0.5
    assert isinstance(m, FBig) and m == 1.5
    m = UBig(1)
    m *= Fraction(1, 2)
    assert isinstance(m, RBig) and m == Fraction(1, 2)


def test_reflected_integer_ops():
    # int on the left of %, divmod, &, |, ^ and ** used to raise TypeError
    import operator
    ops = [operator.mod, divmod, operator.and_, operator.or_, operator.xor,
           operator.add, operator.sub, operator.mul, operator.floordiv]
    for a in (7, 5, 0, -7, 2**70 + 1, -(2**70) - 1):
        for b in (UBig(3), IBig(3), IBig(-3), UBig(2**65 + 1), IBig(-(2**65) - 1)):
            for op in ops:
                expected = op(a, int(b))
                got = op(a, b)
                if op is divmod:
                    assert (int(got[0]), int(got[1])) == expected, (a, b)
                else:
                    assert int(got) == expected, (op.__name__, a, b)
                    # the same type as with the dashu number on the left
                    # (a Python int beyond 64 bits is always taken as an IBig)
                    if abs(a) < 2**63:
                        promoted = UBig(a) if a >= 0 else IBig(a)
                        assert type(got) is type(op(promoted, b)), (op.__name__, a, b)

    assert 2 ** UBig(10) == 1024 and isinstance(2 ** UBig(10), UBig)
    assert (-2) ** IBig(3) == -8
    assert pow(2, UBig(10), 7) == pow(2, 10, 7)
    assert pow(-2, IBig(5), -7) == pow(-2, 5, -7)
    # the exponent may be a dashu integer on either side
    assert UBig(2) ** UBig(10) == 1024 and IBig(-2) ** IBig(3) == -8

    # other numbers on the left of % are promoted like on the right
    assert 7.5 % UBig(2) == UBig(15) / 2.0 % 2 == 1.5
    assert isinstance(Fraction(7, 2) % UBig(2), RBig)
    assert isinstance(Decimal("7.5") % IBig(2), DBig)
    import pytest
    with pytest.raises(TypeError):
        1.5 & UBig(3)


def test_integer_zero_division():
    # division and modulo by an exact zero raise ZeroDivisionError (not a Rust panic)
    import pytest
    import operator
    zeros = [0, UBig(0), IBig(0)]
    for cls in (UBig, IBig):
        for zero in zeros:
            for op in (operator.mod, operator.floordiv, operator.truediv, divmod):
                with pytest.raises(ZeroDivisionError):
                    op(cls(5), zero)
        for op in (operator.mod, operator.floordiv, operator.truediv, divmod):
            with pytest.raises(ZeroDivisionError):
                op(5, cls(0))
        with pytest.raises(ZeroDivisionError):
            cls(5) % 0.0
        with pytest.raises(ZeroDivisionError):
            cls(5) / RBig(0)
        with pytest.raises(ZeroDivisionError):
            Fraction(1, 2) / cls(0)
        # true division by a float zero is a float division: infinity, as for FBig
        assert (cls(5) / 0.0).is_infinite()


def test_pow_zero_modulus():
    # pow(x, e, 0) raises ValueError as for Python's int (not a Rust panic)
    import pytest
    for base in (UBig(2), IBig(2), IBig(-2)):
        for modulus in (0, UBig(0), IBig(0)):
            with pytest.raises(ValueError):
                pow(base, 3, modulus)
    with pytest.raises(ValueError):
        pow(2, UBig(3), 0)
    assert pow(IBig(2), 3, 5) == 3 and pow(IBig(2), 3, -5) == -2
    # a negative result of a UBig power is an IBig rather than an error
    assert pow(UBig(2), 3, -5) == -2
    with pytest.raises(OverflowError):
        UBig(1) ** (2**70)
    with pytest.raises(TypeError):
        UBig(2) ** -1


def test_number_theory_zero_arguments():
    import pytest
    assert UBig(0).gcd(0) == 0 and UBig(0).gcd(6) == 6
    assert tuple(int(x) for x in UBig(0).gcd_ext(0)) == (0, 0, 0)
    assert UBig(0).is_multiple_of(0) and not UBig(5).is_multiple_of(0)
    for n in (UBig(5), IBig(5)):
        with pytest.raises(ValueError):
            n.nth_root(0)
    for n in (UBig(0), IBig(0)):
        with pytest.raises(ValueError):
            n.ilog(2)


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("int math tests passed")
