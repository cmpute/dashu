import math
from decimal import Decimal

import dashu
from dashu import *


def test_construct():
    assert float(FBig(1.5)) == 1.5
    assert FBig(7).to_int() == 7
    assert float(DBig(Decimal("1.5"))) == 1.5
    assert float(DBig("1.25")) == 1.25


def test_arithmetic():
    a = FBig(1.5)
    assert a + 2.0 == FBig(3.5)
    assert a - 2.0 == FBig(-0.5)
    assert a * 2.0 == FBig(3.0)
    assert a / 2.0 == FBig(0.75)
    assert -a == FBig(-1.5)
    assert abs(FBig(-1.5)) == FBig(1.5)
    assert a + 1 == FBig(2.5)


def test_compare_bool():
    assert FBig(1.5) < 2.0
    assert FBig(1.5) <= 1.5
    assert FBig(2.0) > 1.5
    assert FBig(1.5) != 2.0
    assert FBig(1.5) == 1.5
    assert bool(FBig(0.0)) is False
    assert bool(FBig(0.5)) is True


def test_predicates_rounding():
    f = FBig(1.5)
    assert f.is_finite() and not f.is_infinite()
    assert FBig(0.0).is_zero()
    assert f.floor().to_int() == 1
    assert f.ceil().to_int() == 2
    assert f.round().to_int() == 2
    assert f.trunc().to_int() == 1


def test_rounding_dunders():
    # math.floor/ceil/trunc and round() go through the __dunder__ protocol (returning integers)
    # rather than raising TypeError.
    assert math.floor(FBig(2.7)) == 2
    assert math.ceil(FBig(2.1)) == 3
    assert math.trunc(FBig(-2.7)) == -2
    assert round(FBig(2.4)) == 2
    assert round(FBig(2.6)) == 3
    # round(x, n) is decimal-place rounding: supported on the decimal type (via quantize),
    # rejected on the base-2 FBig with a message pointing at the decimal type.
    r2 = round(DBig(Decimal("3.14159")), 2)
    assert abs(float(r2) - 3.14) < 1e-9
    import pytest
    with pytest.raises(ValueError):
        round(FBig(3.14159), 2)


def test_negative_zero_is_zero():
    # -0 is numerically zero: is_zero() is True and bool(-0) is False (matching Python floats).
    assert FBig(-0.0).is_zero()
    assert not bool(FBig(-0.0))


def test_transcendentals_panicfree():
    assert FBig(4.0).sqrt() == 2.0
    assert abs(float(FBig(1.0).sin()) - math.sin(1)) < 1e-9
    assert abs(float(FBig(0.0).sin())) < 1e-9
    assert abs(float(FBig(2.0).ln()) - math.log(2)) < 1e-9
    assert abs(float(FBig(1.0).exp()) - math.e) < 1e-9
    # domain error -> ValueError (not a session-crashing panic)
    try:
        FBig(-1.0).sqrt()
        raise AssertionError("expected ValueError")
    except ValueError:
        pass
    try:
        FBig(2.0).acos()
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_cross_conversions():
    f = FBig(0.5)
    assert float(f.to_decimal()) == 0.5
    assert f.to_binary() == f
    # exact float -> rational, and rational -> float (lossy, with precision)
    assert f.to_rational() == RBig.from_parts(1, 2)
    assert RBig.from_parts(1, 2).to_float(53) == FBig(0.5)


def test_dbig_arithmetic():
    assert float(DBig("1.5") + DBig("1.5")) == 3.0
    assert float(DBig("3.0") * DBig("2.0")) == 6.0
    assert DBig("1.5") < DBig("2.5")


def test_int_truncates():
    # int() truncates toward zero for every type, as for float / Decimal / Fraction
    # (int(DBig) used to round to nearest: int(DBig("2.7")) was 3)
    for text in ["0.4", "0.5", "2.7", "-2.5", "-0.5", "-2.7", "3", "1e3", "12345678901234567890.9"]:
        expected = int(Decimal(text))
        d = DBig(text)
        assert int(d) == expected == math.trunc(d), text
    for value in [0.4, 0.5, 2.7, -2.5, -0.5, 3.0, 1e20]:
        assert int(FBig(value)) == int(value) == math.trunc(FBig(value))
    # to_int() still rounds with the rounding mode of the type
    assert DBig("2.7").to_int() == 3

    import pytest
    for inf in (FBig(float("inf")), FBig(float("-inf")), DBig(1) / 0):
        for convert in (int, math.trunc, math.floor, math.ceil, round):
            with pytest.raises(OverflowError):
                convert(inf)


def test_float_modulo_by_zero():
    # x % 0 raises ZeroDivisionError as for float (not a Rust panic)
    import pytest
    for x in (FBig(5.0), DBig(5)):
        for zero in (0, 0.0, type(x)(0), UBig(0), -0.0):
            with pytest.raises(ZeroDivisionError):
                x % zero
        with pytest.raises(ZeroDivisionError):
            5 % type(x)(0)
        with pytest.raises(ZeroDivisionError):
            UBig(5) % type(x)(0)
        # division by zero keeps giving an infinity
        assert (x / 0).is_infinite()
    assert FBig(5.0) % 3 == 2
    with pytest.raises(ValueError):
        FBig(float("inf")) % 2


def test_mod_is_floored():
    # % is Python's floored modulo for the floats too, whatever the rounding mode of the type
    # (FBig(-5.0) % 3 was -2 and DBig(5) % 3 was -1)
    values = [-7.5, -5.0, -4.0, -0.75, 0.0, 0.75, 4.0, 5.0, 7.5]
    for a in values:
        for b in values:
            if b == 0:
                continue
            assert FBig(a) % FBig(b) == a % b, (a, b)
            assert FBig(a) % b == a % b, (a, b)
            assert a % FBig(b) == a % b, (a, b)
            assert DBig(a) % DBig(b) == a % b, (a, b)
    assert DBig(5) % 3 == UBig(5) % DBig(3) == 5 % DBig(3) == 2
    assert IBig(-5) % FBig(3.0) == 1 and UBig(5) % FBig(-3.0) == -1
    assert DBig("7.5") % 2 == Decimal("1.5")


def test_rounding_methods_keep_a_precision():
    # trunc / floor / ceil / round / fract returned a value of unlimited precision (0) for a
    # zero result and where no digit of the number was left, and the transcendentals then
    # panicked on it: FBig(0.0).fract().acos(), DBig.from_parts(1, -1).ceil().sin()
    methods = ["trunc", "floor", "ceil", "round", "fract"]
    values = [0.0, 0.1, 0.3, 0.5, 0.75, 1.0, 2.5, -0.1, -0.5, -2.5, 1e-10, 123456.75]
    for cls in (FBig, DBig):
        for v in values:
            x = cls(v)
            for m in methods:
                r = getattr(x, m)()
                assert r.precision() > 0, (cls.__name__, v, m)
                # the value is unchanged by the fix, and usable in a transcendental
                if m != "round" and m != "fract":
                    assert r == getattr(math, m)(v), (cls.__name__, v, m)
                r.cos().sin()
                r.exp()
    assert FBig(0.0).fract().precision() == 53
    assert FBig(0.0).fract().acos() == FBig(0.0).acos()
    assert abs(float(FBig(0.1).trunc().acos()) - math.pi / 2) < 1e-15
    one = DBig.from_parts(1, -1).ceil()
    assert one == 1 and one.precision() == 1 and one.sin() == DBig("1").sin()
    assert round(DBig("0.001"), 2).precision() == 4 and round(DBig("0.001"), 2) == 0
    round(DBig("0.001"), 2).cos()
    # a rounding that leaves digits keeps the precision the Rust method gives
    assert DBig("1.234").trunc().precision() == 1


def test_unlimited_precision_raises_instead_of_panicking():
    # a value of unlimited precision (with_precision(0)) cannot be the operand of a function
    # whose result has to be rounded: that is a ValueError, where the binding panicked
    import pytest
    unary = ["sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "asinh",
             "acosh", "atanh", "exp", "exp_m1", "ln", "ln_1p", "log2", "log10", "sqrt", "cbrt",
             "sin_cos", "sinh_cosh"]
    for cls in (FBig, DBig):
        for v in (0.0, 0.5, 1.0, 4.0):
            x = cls(v).with_precision(0)
            assert x.precision() == 0
            for name in unary:
                with pytest.raises(ValueError):
                    getattr(x, name)()
            for call in (lambda: x.nth_root(2), lambda: x.powi(-2), lambda: x.powf(x),
                         lambda: x.atan2(x)):
                with pytest.raises(ValueError):
                    call()
            # exact operations still work
            assert x.powi(2) == v * v and x.powi(2).precision() == 0
            assert (x * x + x).precision() == 0
            # with a limited precision on the other operand there is one to round to
            assert x.atan2(cls(1.0)).precision() == cls(1.0).precision()
    x = FBig(0.5).with_precision(0)
    for fn in (dashu.sin, dashu.sqrt, dashu.exp, dashu.log, dashu.sin_cos):
        with pytest.raises(ValueError):
            fn(x)
    for fn in (dashu.hypot, dashu.atan2, dashu.powf):
        with pytest.raises(ValueError):
            fn(x, x)
        assert fn(x, 0.5).precision() == fn(0.5, x).precision() == 53
    with pytest.raises(ValueError):
        dashu.nth_root(x, 3)
    with pytest.raises(ValueError):
        dashu.powi(x, -1)
    assert dashu.powi(x, 2) == 0.25

    z = CBig(FBig(1.0).with_precision(0), FBig(2.0).with_precision(0))
    assert z.precision() == 0
    for name in ["sin", "cos", "tan", "exp", "ln", "sinh", "cosh", "tanh", "asinh", "acosh",
                 "atanh", "sqrt", "abs", "arg"]:
        with pytest.raises(ValueError):
            getattr(z, name)()
    with pytest.raises(ValueError):
        z.powf(z)
    with pytest.raises(ValueError):
        z.powi(-1)
    assert z.powi(2) == CBig(-3.0, 4.0) and z.norm() == 5


def test_arithmetic_with_infinity_raises():
    # + - * / with an infinite operand raise ValueError (as % and the transcendentals do)
    # instead of a Rust panic, from either side and with any other number
    import operator
    import pytest
    infinities = [FBig(float("inf")), FBig(float("-inf")), FBig(1.0) / 0.0, DBig(1) / 0,
                  -(DBig(1) / 0), FBig(1e300).exp()]
    for inf in infinities:
        assert inf.is_infinite()
        others = [1, -2, 2**70, 1.5, UBig(3), IBig(-3), RBig(2), type(inf)(2), inf, -inf]
        for op in (operator.add, operator.sub, operator.mul, operator.truediv):
            for other in others:
                with pytest.raises(ValueError):
                    op(inf, other)
                # (an RBig on the left cannot take a float infinity either)
                with pytest.raises(ValueError):
                    op(other, inf)
        # an infinity can still be compared, negated and printed
        assert inf == inf and inf != -inf and abs(inf) > 10**100 and "inf" in str(inf)
    for z in (CBig(float("inf"), 2.0), CBig(1.0, float("-inf"))):
        for op in (operator.add, operator.sub, operator.mul, operator.truediv):
            for other in (1, 2.5, CBig(1.0, 1.0), FBig(2.0), z):
                with pytest.raises(ValueError):
                    op(z, other)
                with pytest.raises(ValueError):
                    op(other, z)


def test_zero_over_zero_raises():
    # 0 / 0 is indeterminate: ZeroDivisionError (a Rust panic before), while x / 0 for a
    # float x that is not zero stays an infinity
    import pytest
    for zero in (FBig(0.0), DBig(0), DBig("0.00"), FBig(0)):
        for other in (type(zero)(0), 0, 0.0, UBig(0), IBig(0)):
            with pytest.raises(ZeroDivisionError):
                zero / other
            with pytest.raises(ZeroDivisionError):
                other / zero
        assert (type(zero)(1) / zero).is_infinite()
        assert zero / 2 == 0
    for other in (CBig(0.0, 0.0), 0, 0.0):
        with pytest.raises(ZeroDivisionError):
            CBig(0.0, 0.0) / other
        with pytest.raises(ZeroDivisionError):
            other / CBig(0.0, 0.0)
    assert (CBig(1.0, 0.0) / CBig(0.0, 0.0)).is_infinite()
    # the quotient of two numbers of unlimited precision has no precision to round to
    one, three = FBig(1.0).with_precision(0), FBig(3.0).with_precision(0)
    with pytest.raises(ValueError):
        one / three
    assert (one / FBig(3.0)).precision() == 53


def test_methods_of_infinity_raise():
    # the rounding methods, round(x, n), to_int() and digits() of an infinity raise an
    # ordinary exception (as math.floor and round(x) already did) instead of a Rust panic
    import pytest
    infinities = [FBig(float("inf")), FBig(float("-inf")), DBig(1) / 0, DBig(-1) / 0]
    for inf in infinities:
        for method in ("trunc", "floor", "ceil", "round", "fract", "digits"):
            with pytest.raises(ValueError):
                getattr(inf, method)()
        for fn in (math.floor, math.ceil, math.trunc, round, int, type(inf).to_int):
            with pytest.raises(OverflowError):
                fn(inf)
        # these have a value for an infinity
        assert inf.to_decimal().is_infinite() and inf.to_binary().is_infinite()
        assert inf.with_precision(10).is_infinite() and inf.precision() >= 0
        assert float(inf) == (math.inf if inf > 0 else -math.inf)
    for inf in infinities[2:]:
        with pytest.raises(ValueError):
            round(inf, 2)
    # finite values are unaffected
    assert FBig(2.5).to_int() == 2 and DBig("2.5").to_int() == 3
    assert FBig(2.5).digits() == 3 and round(DBig("1.2345"), 2) == DBig("1.23")


def test_dbig_from_float_infinity():
    # DBig(float('inf')) is the infinity of that sign, like FBig(float('inf')); it raised
    # SyntaxError ("invalid digit for the given radix")
    import pytest
    for v in (float("inf"), float("-inf")):
        d = DBig(v)
        assert d.is_infinite() and d == FBig(v) and d == v and float(d) == v
        assert d == (DBig(1) / 0 if v > 0 else DBig(-1) / 0)
        assert str(d) == str(v)
        # a float infinity mixed into decimal arithmetic is an infinite operand
        with pytest.raises(ValueError):
            DBig(1) + v
        with pytest.raises(ValueError):
            v * DBig(2)
        assert (DBig(1) < v) == (1 < v)
    with pytest.raises(ValueError):
        DBig(float("nan"))
    assert DBig(-0.0) == 0 and DBig(1.5).precision() == 2


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("float ops tests passed")
