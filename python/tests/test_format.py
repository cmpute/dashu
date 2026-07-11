import math

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


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("format tests passed")
