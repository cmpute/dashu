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
