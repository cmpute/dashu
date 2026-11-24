from dashu import *

def test_bit_ops():
    ##### getters #####
    n = UBig(12) # 0b1100
    assert not n[0] and not n[1] and n[2] and n[3]
    assert int(n[:2]) == 0 and int(n[2:]) == 3
    assert int(n[:3]) == 4 and int(n[3:]) == 1

    ##### setters #####
    n = UBig(12)
    n[0] = True
    assert int(n) == 0b1101
    n[10] = True
    assert int(n) == 0b10000001101

    n = UBig(12)
    n[:2] = True
    assert int(n) == 0b1111
    n[2:] = False
    assert int(n) == 0b0011

    n = UBig(12)
    n[1:3] = True
    assert int(n) == 0b1110

    ##### delete #####
    n = UBig(12)
    del n[0]
    assert int(n) == 0b110
    
    n = UBig(12)
    del n[2]
    assert int(n) == 0b100

    n = UBig(12)
    del n[:2]
    assert int(n) == 0b11

    n = UBig(12)
    del n[2:]
    assert int(n) == 0b00
    
    n = UBig(12)
    del n[1:3]
    assert int(n) == 0b10

    ## misc
    n = UBig("0x3bc0495b81ab5b422d2b18c7e61a2309cf548cda8fbbd18d41aded48711b72d6")
    assert n[:32] == n & (2**32 - 1)
    assert n[:64] == n & (2**64 - 1)
    assert n[:128] == n & (2**128 - 1)

def test_conversion_to_chunks():
    n = UBig("0x123456789abcdef")
    assert UBig.from_chunks(n.to_chunks(10), 10) == n

def _bits(v):
    return [(v >> k) & 1 for k in range(v.bit_length())]


def _from_bits(bits):
    return sum(b << k for k, b in enumerate(bits))


def test_del_bits_matches_list_model():
    # A UBig is a little-endian list of bits: `del n[key]` must agree with `del` on that list.
    # (The bits below the deleted range used to be replaced by the deleted ones.)
    cases = [
        (12, 0), (1, 0), (13, 0), (15, 2), (0b1011, 1), (0b1011, -1), (0b110101, 3),
        (3, slice(1, 1)), (12, slice(1, 3)), (0b110101, slice(2, 4)), (0b110101, slice(1, -1)),
        (0b110101, slice(0, 2)), (0b110101, slice(3, None)), (0b110101, slice(None, None)),
        # empty slices, including stop < start, delete nothing
        (12, slice(3, 1)), (0b110101, slice(4, 2)), (0b110101, slice(-1, 1)), (5, slice(7, 9)),
        (2**70 + 5, 0), (2**70 + 5, 64), (2**130 + 2**64 + 3, slice(1, 65)),
    ]
    for v, key in cases:
        n = UBig(v)
        del n[key]
        expected = _bits(v)
        del expected[key]
        assert int(n) == _from_bits(expected), (v, key)


def test_set_bit_slice_matches_list_model():
    # an empty slice (also with stop < start, which used to panic) selects no bits
    cases = [
        (12, slice(1, 1)), (12, slice(3, 1)), (12, slice(1, 3)), (12, slice(None, 2)),
        (12, slice(2, None)), (0b110101, slice(4, 2)), (0b110101, slice(-1, 1)),
        (0b110101, slice(1, -1)), (2**70 + 5, slice(66, 3)), (2**70 + 5, slice(3, 66)),
    ]
    for v, key in cases:
        for value in (True, False):
            n = UBig(v)
            n[key] = value
            expected = _bits(v)
            expected[key] = [int(value)] * len(expected[key])
            assert int(n) == _from_bits(expected), (v, key, value)


def test_bit_index_out_of_index_range():
    # an integer index that does not fit in an index-sized integer is an IndexError (as for
    # a list), not a TypeError claiming that the index is not an integer
    import pytest
    for index in (2**63, 2**64, -2**63 - 1):
        with pytest.raises(IndexError):
            UBig(5)[index]
        with pytest.raises(IndexError):
            IBig(5)[index]
        with pytest.raises(IndexError):
            UBig(5)[index] = True
        with pytest.raises(IndexError):
            del UBig(5)[index]
    with pytest.raises(TypeError):
        UBig(5)["a"]


if __name__ == "__main__":
    # test_bit_ops()
    test_conversion_to_chunks()
