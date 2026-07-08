import math

from dashu import *


def test_float_scientific():
    assert format(FBig(1.5), "e") == "1.500000e+00"
    assert format(FBig(1.5), ".2e") == "1.50e+00"
    assert format(FBig(12345.0), "E") == "1.234500E+04"
    assert format(FBig(0.5), ".4f") == "0.5000"
    # arbitrary precision is preserved through formatting
    assert format(FBig(2.0).with_precision(200).exp(), ".20e") == "7.38905609893065022723e+00"


def test_float_layout():
    assert format(FBig(3.14), "010.2f") == "0000003.14"
    assert format(FBig(1234567.0), ",.2f") == "1,234,567.00"
    assert format(FBig(5.0), "+.1f") == "+5.0"
    assert format(FBig(5.0), "^10") == "    5.0   " or format(FBig(5.0), "^10") == "    5     "


def test_dbig_format():
    assert format(DBig("1.5"), ".3e") == "1.500e+00"
    assert format(DBig("1.5"), "") == "1.5"


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
