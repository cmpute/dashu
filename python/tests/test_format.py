import math
from fractions import Fraction

from dashu import *


def test_fbig_hex_default():
    # FBig is base 2: default str / format is lossless hexadecimal (no base conversion)
    assert str(FBig(1.5)) == "0x3p-1"  # 3 * 2^-1
    assert str(FBig(2.0)) == "0x1p1"
    assert str(FBig(0.5)) == "0x1p-1"
    assert format(FBig(1.5), "") == "0x3p-1"
    assert format(FBig(1.5), "a") == "0x3p-1"
    assert format(FBig(1.5), ".4a") == "0x3.0000p-1"


def test_float_scientific_native_default():
    # 'e' default shows the value's full native precision (not CPython's fixed 6 digits)
    assert format(FBig(1.5), "e") == "1.5e+00"
    assert format(DBig("1.5"), "e") == "1.5e+00"
    assert format(FBig(12345.0), "E") == "1.2345E+04"
    # explicit precision still overrides
    assert format(FBig(1.5), ".2e") == "1.50e+00"
    assert format(FBig(0.5), ".4f") == "0.5000"
    # a high-precision value prints all its significant digits by default
    f = DBig("1.23").with_precision(100).powi(100000)
    assert format(f, "e").startswith("3.2444713221954405338085664273496250244079439837529295953908288")
    # arbitrary precision is preserved through explicit formatting too
    assert format(FBig(2.0).with_precision(200).exp(), ".20e") == "7.38905609893065022723e+00"


def test_float_layout():
    assert format(FBig(3.14), "010.2f") == "0000003.14"
    assert format(FBig(1234567.0), ",.2f") == "1,234,567.00"
    assert format(FBig(5.0), "+.1f") == "+5.0"
    assert format(DBig("123.5"), ">8") == "   123.5"


def test_dbig_format():
    # DBig is base 10: default str is plain decimal
    assert str(DBig("1.5")) == "1.5"
    assert format(DBig("1.5"), ".3e") == "1.500e+00"


def test_integers_delegate():
    assert format(UBig(255), "#x") == "0xff"
    assert format(UBig(5), "b") == "101"
    assert format(IBig(-42), "08d") == "-0000042"
    assert format(UBig(10**9), ",") == "1,000,000,000"


def test_rational_complex():
    assert format(RBig.from_parts(1, 3), ".4f") == "0.3333"
    assert format(CBig(3.0, 4.0), ".2f") == "(3.00+4.00j)"
    # default spec for rational is the fraction
    assert "1/3" in format(RBig.from_parts(1, 3), "")


def test_fbig_decimal_digits_are_exact():
    # the decimal presentations print the digits of the exact binary value (they used to stop
    # at the ~15 digits that 53 bits are worth, printing zeros after them)
    x = 1000000000000001.0
    assert format(FBig(x), ".0f") == "1000000000000001" == format(x, ".0f")
    assert format(FBig(x), "f") == format(x, "f")
    assert format(FBig(1e22), ".0f") == format(1e22, ".0f")
    assert format(CBig(0.0, x), ".0f") == "(0+1000000000000001j)"
    y = 0.1 + 0.2
    assert format(FBig(y), ".16e") == "3.0000000000000004e-01" == format(y, ".16e")
    assert format(FBig(y), ".17f") == "0.30000000000000004" == format(y, ".17f")
    assert format(FBig(0.1), ".30f") == format(0.1, ".30f")
    assert format(FBig(0.1), ".25e") == format(0.1, ".25e")
    assert format(FBig(1e-300), ".20e") == format(1e-300, ".20e")
    assert format(FBig(2.0).with_precision(200).exp(), ".20e") == "7.38905609893065022723e+00"
    # without a precision, 'e' / 'g' show the digits that the binary precision is worth
    assert format(FBig(0.1), "e") == "1e-01"
    assert format(FBig(0.1), "g") == "0.1"
    # infinities
    assert format(FBig(float("inf")), ".2f") == "inf"
    assert format(FBig(float("-inf")), "e") == "-inf"
    assert format(FBig(float("inf")), "+08.2F") == "+0000INF"


def test_rational_digits_are_exact():
    # the digits of an RBig are those of the exact value, rounded once at the requested
    # precision (they used to be wrong beyond 16 digits)
    cases = [
        (Fraction(10**15 + 1), ".0f"),
        (Fraction(10**16 + 1), ".0f"),
        (Fraction(3 * 10**20 + 1, 3), ".4f"),
        (Fraction(1, 3), ".4f"),
        (Fraction(12499999999999999999, 10**20), ".2f"),
        (Fraction(12499, 10**5), ".2f"),
        (Fraction(-(10**30) - 1, 7), ".3f"),
        (Fraction(1, 3), ".40f"),
        (Fraction(2, 3), ".1%"),
        (Fraction(10**20 + 1, 3), ".25e"),
        (Fraction(351521574481754164273937838059, 4), ".27e"),
        (Fraction(1, 3 * 10**30), ".20e"),
        (Fraction(2, 3), ".5g"),
        (Fraction(0), ".2f"),
        (Fraction(0), ".2e"),
    ]
    for value, spec in cases:
        assert format(RBig(value), spec) == format(value, spec), (value, spec)
    assert format(RBig.from_parts(1, 3), "f") == "0.333333"
    assert format(RBig.from_parts(999, 1000), ".2f") == "1.00"
    # ties round away from zero, like RBig.to_decimal and DBig
    assert format(RBig.from_parts(1, 8), ".2f") == "0.13"


def test_general_format():
    # 'g' / 'G' use the scientific notation when the exponent (after rounding) is below -4
    # or not below the precision, as in Python; they never did before
    cases = [
        ("10", ".1g"), ("123456", ".2G"), ("1e30", ".3g"), ("12", ".2g"), ("1.5", ".3g"),
        ("0.00012345", ".2g"), ("0.000012345", ".2g"), ("99.9", ".2g"), ("9.99", ".2g"),
        ("123456", ".6g"), ("1234567", ".6g"), ("0.5", ".1g"), ("1500", ".2g"), ("-1500", ".2g"),
        ("100", ".3g"), ("100", ".1g"), ("0", ".3g"), ("1e-5", ".3g"), ("1e-4", ".3g"),
    ]
    for text, spec in cases:
        expected = format(float(text), spec)
        assert format(DBig(text), spec) == expected, (text, spec)
        assert format(FBig(float(text)), spec) == expected, (text, spec)
    # with no precision, all the digits of the value are significant
    assert format(DBig("1.5e-20"), "g") == "1.5e-20"
    assert format(DBig("1e30"), "g") == "1e+30"
    assert format(DBig("123456789"), "g") == "123456789"
    assert format(DBig("100"), "g") == "100"
    assert format(DBig("1.50"), "g") == "1.5"
    assert format(DBig("12345"), ",g") == "12,345"
    assert format(DBig("12345"), "010,.3g") == format(12345.0, "010,.3g") == "001.23e+04"


def test_hex_format_sign_and_padding():
    # the sign option applies to the hexadecimal presentations too, and the zero-padding goes
    # between the 0x prefix and the digits, so that the result can be read back
    x = FBig(1.5)
    assert format(x, "+") == "+0x3p-1" == format(x, "+a")
    assert format(x, " a") == " 0x3p-1"
    assert format(x, "-a") == "0x3p-1"
    assert format(FBig(-1.5), "+a") == "-0x3p-1"
    assert format(x, "+A") == "+" + format(x, "A")
    padded = format(FBig(-1.5), "012a")
    assert padded == "-0x000003p-1"
    assert FBig(padded) == FBig(-1.5)
    assert FBig(format(x, "+012a")) == x and format(x, "+012a") == "+0x000003p-1"
    assert format(FBig(-1.5), "*=12a") == "-0x*****3p-1"
    assert format(FBig(-1.5), ">12a") == "     -0x3p-1"
    assert format(FBig(-1.5), "<12a") == "-0x3p-1     "
    assert format(x, "+e") == "+1.5e+00"


def test_decimal_layout_matches_float():
    # sign, zero-padding and grouping are laid out as for float: the padding zeros are grouped
    # too, and the ' ' sign is not a digit
    cases = [
        ("0", "05,.0f"), ("1234.5", "012,.2f"), ("123456", " ,.0f"), ("5", " 08.2f"),
        ("-1234.5", "012,.2f"), ("1234.5", ",.2f"), ("5", "+08.2f"), ("1", "04,.0f"),
        ("1", "06,.0f"), ("1234567", "_.0f"), ("1234567", "015_.1f"), ("1234.5", "x>12,.2f"),
        ("1234.5", "0=12,.2f"), ("1234.5", "0>12,.2f"), ("1234.5", "012,.2e"), ("-5", "x=8.1f"),
        ("-5", " 8.1f"), ("5", " <8.1f"), ("5", "^9.1f"), ("5", "<9.1f"), ("5", "+,.1f"),
        ("1234567.5", " ,.1%"), ("123456789", "+020,.2f"), ("123456789", " 020_.2f"),
        ("0.25", "08,.2f"), ("-0.25", "08,.2f"),
    ]
    for text, spec in cases:
        expected = format(float(text), spec)
        assert format(DBig(text), spec) == expected, (text, spec)
        assert format(FBig(float(text)), spec) == expected, (text, spec)


def test_rational_negative_rounding_to_zero_keeps_sign():
    # a negative RBig that rounds to zero in a fixed notation prints as '-0.00', as Fraction,
    # float and DBig do (the sign was dropped)
    cases = [
        (Fraction(-1, 1000), ".2f"),
        (Fraction(-1, 3), ".0f"),
        (Fraction(-1, 100000), ".1%"),
        (Fraction(-1, 10**30), "f"),
        (Fraction(-1, 1000), "+.2F"),
        (Fraction(-1, 1000), "08.2f"),
        (Fraction(-1, 1000), ",.2f"),
        # neighbours that do not round to zero, and zero itself
        (Fraction(-1, 100), ".2f"),
        (Fraction(-3, 500), ".2f"),
        (Fraction(-1, 250), ".2f"),
        (Fraction(1, 1000), ".2f"),
        (Fraction(0), ".2f"),
    ]
    for value, spec in cases:
        assert format(RBig(value), spec) == format(value, spec), (value, spec)
    assert format(RBig(Fraction(-1, 1000)), ".2f") == format(DBig("-0.001"), ".2f") == "-0.00"


def test_explicit_fill_with_leading_zero_width():
    # after an explicit fill and alignment, a `0` is only the first digit of the width (as
    # for float): the fill character and the alignment were replaced by zero-padding
    cases = [
        (1.5, "x<010.2f"), (1.5, "x>010.2f"), (1.5, "x^010.2f"), (-2.5, "*>010.1f"),
        (-2.5, "*=+010.1f"), (-2.5, "0<010.1f"), (1.5, " >08.3e"), (1234.5, "_<012,.1f"),
        (1.5, "x<0.2f"), (1.5, "x<00010.2f"),
        # without an explicit fill the `0` is the zero-padding flag, whatever the alignment
        (1.5, "<010.2f"), (-1.5, ">010.2f"), (-1.5, "^010.2f"), (-1.5, "010.2f"),
        (-1.5, "=010.2f"), (1.5, "x<10.2f"),
    ]
    for value, spec in cases:
        want = format(value, spec)
        assert format(DBig(str(value)), spec) == want, (value, spec)
        assert format(FBig(value), spec) == want, (value, spec)
        assert format(RBig(value), spec) == want, (value, spec)
    assert format(DBig("1.5"), "x<010.2f") == "1.50xxxxxx"
    assert format(DBig("-2.5"), "*>010.1f") == "******-2.5"
    # the hexadecimal presentation of FBig follows the same rule
    assert format(FBig(-1.5), "x>012a") == "xxxxx-0x3p-1"
    assert format(FBig(-1.5), "012a") == "-0x000003p-1"


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("format tests passed")
