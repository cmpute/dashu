import pytest

dashu = pytest.importorskip("dashu")
if not hasattr(dashu, "serde"):
    pytest.skip("dashu not built with the serde feature", allow_module_level=True)

from dashu import serde
from dashu import UBig, IBig, FBig, DBig, RBig, CBig


def test_json_roundtrip_ubig():
    v = UBig(3) ** 300
    s = serde.to_json(v)
    assert isinstance(s, str)
    assert serde.from_json(UBig, s) == v


def test_json_roundtrip_ibig():
    v = IBig(-(3**250))
    assert serde.from_json(IBig, serde.to_json(v)) == v


def test_json_roundtrip_fbig():
    v = FBig(3.14159)
    assert serde.from_json(FBig, serde.to_json(v)) == v


def test_json_roundtrip_dbig():
    v = DBig("2.718281828")
    assert serde.from_json(DBig, serde.to_json(v)) == v


def test_json_roundtrip_rbig():
    v = RBig.from_parts(22, 7)
    assert serde.from_json(RBig, serde.to_json(v)) == v


def test_json_roundtrip_cbig():
    v = CBig(FBig(0.5), FBig(0.25))
    w = serde.from_json(CBig, serde.to_json(v))
    assert w == v
    assert w.real() == v.real() and w.imag() == v.imag()


def test_json_wrong_cls():
    with pytest.raises(TypeError):
        serde.from_json(int, "5")


def test_json_invalid():
    with pytest.raises(ValueError):
        serde.from_json(UBig, "not a number")


def test_json_non_dashu_obj():
    with pytest.raises(TypeError):
        serde.to_json([1, 2, 3])


def test_binary_roundtrip_ubig():
    v = UBig(2) ** 500 + UBig(12345)
    data = serde.serialize(v)
    assert isinstance(data, bytes)
    assert serde.deserialize(UBig, data) == v


def test_binary_roundtrip_all_types():
    cases = [
        (UBig, UBig(7) ** 64),
        (IBig, IBig(-(7**64))),
        (FBig, FBig(1.5)),
        (DBig, DBig("9.99")),
        (RBig, RBig.from_parts(1, 3)),
        (CBig, CBig(FBig(1.0), FBig(-2.0))),
    ]
    for cls, v in cases:
        data = serde.serialize(v)
        assert serde.deserialize(cls, data) == v, f"binary round-trip failed for {cls.__name__}"


def test_binary_wrong_cls():
    with pytest.raises(TypeError):
        serde.deserialize(int, b"\x05")


def test_binary_corrupt():
    # byte-strings serialize as opaque blobs (no checksum), so the detectable corruption
    # is a length mismatch: truncating makes the declared varint length exceed the data.
    v = UBig(12345)
    data = serde.serialize(v)
    with pytest.raises(ValueError):
        serde.deserialize(UBig, data[:-1])
