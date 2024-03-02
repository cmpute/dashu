from dashu import *

def test_int_conversions():
    # small cases
    test_cases = [
        (0, UBig),
        (1, UBig),
        (-1, IBig),
        (0xffffffffffffffffffff, UBig),
        (-0xffffffffffffffffffff, IBig),
    ]

    # large cases
    for i in range(6):
        v = (-3)**(9**i)
        test_cases.append((v, UBig if v >= 0 else IBig))

    # testing
    for v, t in test_cases:
        if v < 0: # test constructors
            _ = IBig(v)
        else:
            _ = UBig(v), IBig(v)
        assert type(auto(v)) == t # test result type of auto

if __name__ == "__main__":
    test_int_conversions()