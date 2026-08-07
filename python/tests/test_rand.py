import pytest

dashu = pytest.importorskip("dashu")
if not hasattr(dashu, "rand"):
    pytest.skip("dashu not built with the rand feature", allow_module_level=True)

from dashu import rand
from dashu import UBig, IBig, FBig, DBig, RBig, CBig


def test_ubig_bits():
    for bits in (1, 8, 64, 65, 128, 1000):
        v = rand.ubig(bits=bits)
        assert isinstance(v, UBig)
        assert len(v) <= bits
        assert v >= 0


def test_ubig_zero_bits():
    assert rand.ubig(bits=0) == UBig(0)


def test_ibig_bits():
    for bits in (1, 8, 64, 65, 200):
        v = rand.ibig(bits=bits)
        assert isinstance(v, IBig)
        assert len(v) <= bits + 1  # sign not counted in magnitude


def test_fbig_unit_interval():
    for _ in range(5):
        v = rand.fbig()
        assert isinstance(v, FBig)
        assert 0 <= v < 1


def test_fbig_precision():
    v = rand.fbig(precision=100)
    assert v.precision() == 100


def test_dbig_unit_interval():
    for _ in range(5):
        v = rand.dbig()
        assert isinstance(v, DBig)
        assert 0 <= v < 1


def test_rbig_unit_interval():
    for _ in range(5):
        v = rand.rbig()
        assert isinstance(v, RBig)
        assert 0 <= v < 1


def test_rbig_denom_bits():
    v = rand.rbig(max_denom_bits=8)
    assert len(v.denominator) <= 8


def test_cbig_unit_square():
    for _ in range(5):
        v = rand.cbig()
        assert isinstance(v, CBig)
        assert 0 <= v.real() < 1
        assert 0 <= v.imag() < 1


def test_bad_precision():
    with pytest.raises(ValueError):
        rand.fbig(precision=0)
    with pytest.raises(ValueError):
        rand.cbig(precision=0)
