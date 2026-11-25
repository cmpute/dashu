import pytest

dashu = pytest.importorskip("dashu")
if not hasattr(dashu.UBig, "zeroize"):
    pytest.skip("dashu not built with the zeroize feature", allow_module_level=True)

from dashu import UBig, IBig, FBig, DBig, RBig, CBig


def test_zeroize_ubig():
    v = UBig(12345)
    v.zeroize()
    assert v == UBig(0)


def test_zeroize_ibig():
    v = IBig(-12345)
    v.zeroize()
    assert v == IBig(0)


def test_zeroize_fbig():
    v = FBig(3.14)
    v.zeroize()
    assert v == FBig(0)


def test_zeroize_dbig():
    v = DBig("3.14")
    v.zeroize()
    assert v == DBig(0)


def test_zeroize_rbig():
    v = RBig.from_parts(3, 4)
    v.zeroize()
    assert v == RBig(0)


def test_zeroize_cbig():
    v = CBig(FBig(1.5), FBig(2.5))
    v.zeroize()
    assert v == CBig(0)


def test_zeroized_float_keeps_its_precision():
    # zeroize() left a zero of unlimited precision (0) behind, on which the transcendentals
    # panicked: a zeroized float behaves like a fresh zero of the same precision
    fresh = FBig(0.0)
    x = FBig(3.14)
    x.zeroize()
    assert x == fresh and x.precision() == fresh.precision() == 53
    assert x.cos().sin() == fresh.cos().sin()
    assert x.cos().sqrt() == fresh.cos().sqrt()

    d = DBig("3.14")
    d.zeroize()
    assert d == 0 and d.precision() == 3
    assert d.cos().sin() == DBig("0.00").cos().sin()

    c = CBig(FBig(1.5), FBig(2.5))
    c.zeroize()
    assert c == CBig(0) and c.precision() == 53
    assert c.exp() == CBig(0.0, 0.0).exp()
