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

if __name__ == "__main__":
    # test_bit_ops()
    test_conversion_to_chunks()
