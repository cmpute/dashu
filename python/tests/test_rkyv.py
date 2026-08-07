import pytest

dashu = pytest.importorskip("dashu")
if not hasattr(dashu, "rkyv"):
    pytest.skip("dashu not built with the rkyv feature", allow_module_level=True)

from dashu import rkyv
from dashu import UBig, IBig, FBig, DBig, RBig, CBig


def test_roundtrip_ubig():
    v = UBig(3) ** 500
    data = rkyv.to_bytes(v)
    assert isinstance(data, bytes)
    assert rkyv.from_bytes(UBig, data) == v


def test_roundtrip_ibig():
    v = IBig(-(3**500))
    assert rkyv.from_bytes(IBig, rkyv.to_bytes(v)) == v


def test_roundtrip_fbig():
    v = FBig(1.23456789)
    assert rkyv.from_bytes(FBig, rkyv.to_bytes(v)) == v


def test_roundtrip_dbig():
    v = DBig("9.876543210")
    assert rkyv.from_bytes(DBig, rkyv.to_bytes(v)) == v


def test_roundtrip_rbig():
    v = RBig.from_parts(22, 7)
    assert rkyv.from_bytes(RBig, rkyv.to_bytes(v)) == v


def test_roundtrip_cbig():
    v = CBig(FBig(0.5), FBig(-0.25))
    w = rkyv.from_bytes(CBig, rkyv.to_bytes(v))
    assert w == v
    assert w.real() == v.real() and w.imag() == v.imag()


def test_wrong_cls():
    v = UBig(1)
    data = rkyv.to_bytes(v)
    with pytest.raises(TypeError):
        rkyv.from_bytes(int, data)


def test_non_dashu_obj():
    with pytest.raises(TypeError):
        rkyv.to_bytes([1, 2, 3])
