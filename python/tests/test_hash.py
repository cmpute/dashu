"""Equal numbers have equal hashes, across the dashu types and Python's own number types."""

from decimal import Decimal
from fractions import Fraction

from dashu import *

INTS = [0, 1, -1, -2, 5, -5, 2**61 - 1, 2**61, -(2**61 - 1), 2**64, 2**200 + 7, -(2**200) - 7]
FLOATS = [0.0, -0.0, 1.5, -1.5, 0.1, 1e300, -1e-300, 5e-324, 2.0**61, float("inf"), float("-inf")]
FRACTIONS = [
    Fraction(3, 2), Fraction(-1, 3), Fraction(1, 2**61 - 1), Fraction(-7, 2**61 - 1),
    Fraction(10**40, 3), Fraction(5), Fraction(0),
]
DECIMALS = ["1.5", "-1.25e-30", "1e400", "0.1", "-0", "123456789012345678901234567890", "1e-400"]


def test_hash_matches_python_numbers():
    for i in INTS:
        assert IBig(i) == i and hash(IBig(i)) == hash(i), i
        if i >= 0:
            assert UBig(i) == i and hash(UBig(i)) == hash(i), i
    for f in FLOATS:
        assert FBig(f) == f and hash(FBig(f)) == hash(f), f
    for q in FRACTIONS:
        assert RBig(q) == q and hash(RBig(q)) == hash(q), q
    for text in DECIMALS:
        d = Decimal(text)
        assert DBig(d) == d and hash(DBig(d)) == hash(d), text


def test_hash_matches_across_dashu_types():
    assert hash(UBig(5)) == hash(IBig(5)) == hash(FBig(5.0)) == hash(DBig(5)) == hash(RBig(5))
    assert hash(FBig(1.5)) == hash(DBig("1.5")) == hash(RBig(Fraction(3, 2))) == hash(1.5)
    assert hash(DBig("0.1")) == hash(RBig(Fraction(1, 10))) == hash(Fraction(1, 10))
    assert hash(FBig(-0.0)) == hash(FBig(0.0)) == hash(UBig(0)) == 0


def test_complex_hash():
    for c in [1.5 + 2.5j, -1.5j, 0j, 1e300 - 1e-300j, complex(float("inf"), -2.0), 3 + 0j]:
        assert CBig(c) == c and hash(CBig(c)) == hash(c), c
    # a complex number with a zero imaginary part hashes like its real part
    assert CBig(1.5, 0.0) == FBig(1.5) and hash(CBig(1.5, 0.0)) == hash(FBig(1.5)) == hash(1.5)
    assert hash(CBig(-1.0, 0.0)) == hash(-1) == hash(IBig(-1))
    # the two zeros are equal
    assert CBig(0.0, 0.0) == CBig(-0.0, 0.0) and hash(CBig(0.0, 0.0)) == hash(CBig(-0.0, 0.0))


def test_lookup_in_dict_and_set():
    table = {5: "int", 1.5: "float", Fraction(1, 3): "fraction", Decimal("0.1"): "decimal", 2j: "complex"}
    assert table[UBig(5)] == table[IBig(5)] == "int"
    assert table[FBig(1.5)] == table[DBig("1.5")] == "float"
    assert table[RBig.from_parts(1, 3)] == "fraction"
    assert table[DBig("0.1")] == "decimal"
    assert table[CBig(0.0, 2.0)] == "complex"
    assert 1.5 in {FBig(1.5)} and 5 in {UBig(5)} and Fraction(3, 2) in {RBig.from_parts(3, 2)}
    assert len({UBig(5), IBig(5), 5, FBig(5.0), 5.0, RBig(5), DBig(5)}) == 1


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("hash tests passed")
