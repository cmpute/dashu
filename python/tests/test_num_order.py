from dashu import *

import pytest


def test_compare_ubig_int():
    assert compare(UBig(10), 5) == 1
    assert compare(UBig(10), 10) == 0
    assert compare(UBig(10), 15) == -1


def test_compare_ibig_negative():
    assert compare(IBig(-3), UBig(2)) == -1
    assert compare(IBig(-3), -3) == 0
    assert compare(IBig(0), 0) == 0


def test_compare_fbig_float_exact():
    # FBig vs native float, exact comparison (no lossy f64 intermediate on the FBig side)
    assert compare(FBig(0.5), 0.5) == 0
    assert compare(FBig(0.5), 1.0) == -1
    assert compare(FBig(1.1), 1.0) == 1


def test_compare_huge_int_not_lossy():
    # 2**200 compared exactly against a float, no precision loss
    huge = UBig(2) ** 200
    assert compare(huge, 1e60) == 1
    assert compare(huge, float(2**200)) == 0


def test_compare_rbig():
    assert compare(RBig.from_parts(1, 3), RBig.from_parts(1, 2)) == -1
    assert compare(RBig.from_parts(1, 3), RBig.from_parts(1, 3)) == 0
    assert compare(RBig.from_parts(1, 2), 0.5) == 0
    assert compare(0.5, RBig.from_parts(1, 2)) == 0
    # primitive-vs-rational ordering must be reversed correctly (not equal here)
    assert compare(0.25, RBig.from_parts(1, 2)) == -1
    assert compare(RBig.from_parts(1, 2), 0.25) == 1


def test_compare_both_primitives():
    assert compare(3, 5) == -1
    assert compare(5, 3) == 1
    assert compare(3.0, 3) == 0
    assert compare(3, 3.5) == -1


def test_compare_complex_raises():
    # complex numbers have no ordering
    with pytest.raises(TypeError):
        compare(CBig(1, 2), 1)
    with pytest.raises(TypeError):
        compare(1, CBig(1, 2))


def test_min_max():
    assert min(UBig(3), 5) == 3
    assert max(UBig(3), 5) == 5
    assert min(IBig(-2), UBig(5)) == -2
    assert max(FBig(0.25), 0.5) == 0.5
    assert min(RBig.from_parts(1, 2), RBig.from_parts(1, 3)) == RBig.from_parts(1, 3)
    # min/max return the original operand; the dashu type is preserved when it wins
    assert isinstance(min(UBig(5), 10), UBig)
    assert isinstance(max(3, UBig(5)), UBig)
    assert max(UBig(3), 5) == 5
    assert min(3, UBig(5)) == 3
