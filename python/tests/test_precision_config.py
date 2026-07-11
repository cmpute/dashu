import pytest

import dashu
from dashu import *

DEFAULT = 53  # f64::MANTISSA_DIGITS


@pytest.fixture
def restore_precision():
    """Save and restore the module default precision around each test."""
    original = dashu.get_precision()
    yield original
    dashu.set_precision(original)


def test_default_precision(restore_precision):
    assert restore_precision == DEFAULT
    assert dashu.get_precision() == DEFAULT
    assert FBig(1.5).precision() == DEFAULT
    assert CBig(1.5, 0.5).precision() == DEFAULT
    assert CBig(complex(1.5, 0.5)).precision() == DEFAULT


def test_set_precision_returns_previous(restore_precision):
    assert dashu.set_precision(100) == DEFAULT
    assert dashu.get_precision() == 100


def test_float_construction_honors_precision(restore_precision):
    dashu.set_precision(100)
    assert FBig(1.5).precision() == 100
    assert CBig(1.5, 0.5).precision() == 100
    assert CBig(complex(1.5, 0.5)).precision() == 100
    # transcendentals run at the default precision too
    assert FBig(1.5).exp().precision() == 100


def test_mixed_float_arithmetic_honors_precision(restore_precision):
    dashu.set_precision(80)
    assert (UBig(2) + 3.0).precision() == 80


def test_integer_input_is_unaffected(restore_precision):
    dashu.set_precision(100)
    # integer construction stays exact (its own bit length), not the configured default
    assert FBig(2).precision() != 100
    assert FBig(2) == FBig(2)


def test_zero_precision_rejected(restore_precision):
    with pytest.raises(ValueError):
        dashu.set_precision(0)
