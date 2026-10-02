from dashu import *

slice_list = [
    slice(None, 0), # [:0]
    slice(None, 1), # [:1]
    slice(None, -1), # [:-1]
    slice(0, None), # [0:]
    slice(1, None), # [1:]
    slice(-1, None), # [-1:]

    slice(None, None, 2), # [::2]
    slice(None, 0, 2), # [:0:2]
    slice(None, 1, 2), # [:1:2]
    slice(None, -1, 2), # [:-1:2]
    slice(0, None, 2), # [0::2]
    slice(1, None, 2), # [1::2]
    slice(-1, None, 2), # [-1::2]

    slice(None, None, -2), # [::-2]
    slice(None, 0, -2), # [:0:-2]
    slice(None, 1, -2), # [:1:-2]
    slice(None, -1, -2), # [:-1:-2]
    slice(0, None, -2), # [0::-2]
    slice(1, None, -2), # [1::-2]
    slice(-1, None, -2), # [-1::-2]
]

def test_words_get():
    n = UBig(3 ** 300)
    words = n.to_words()
    words_list = list(words)
    
    # single index
    assert words[0] == words_list[0]
    assert words[1] == words_list[1]
    assert words[-1] == words_list[-1]

    # slice index
    for sl in slice_list:
        assert list(words[sl]) == words_list[sl], "{} => {}, {}".format(sl, words, words_list)

def test_words_set():
    n = UBig(3 ** 300)
    
    # single index
    words = n.to_words()
    words_list = list(words)
    words[0], words_list[0] = 0, 0
    words[1], words_list[1] = 1, 1
    words[-1], words_list[-1] = 2, 2
    assert list(words) == words_list

    # slice index
    for sl in slice_list:
        words = n.to_words()
        words_list = list(words)
        values = list(range(len(words)))

        words[sl] = values[sl]
        words_list[sl] = values[sl]
        assert list(words) == words_list, "{} => {}, {}".format(sl, words, words_list)

def test_words_del():
    n = UBig(3 ** 300)
    
    # single index
    words = n.to_words()
    words_list = list(words)
    del words[0]; del words_list[0]
    del words[1]; del words_list[1]
    del words[-1]; del words_list[-1]
    assert list(words) == words_list

    # slice index
    for sl in slice_list:
        words = n.to_words()
        words_list = list(words)

        del words[sl]; del words_list[sl]
        assert list(words) == words_list, "{} => {}, {}".format(sl, words, words_list)

def test_words_sign_roundtrip():
    values = [IBig(0), IBig(1), IBig(-1), IBig(3 ** 300), IBig(-(3 ** 300))]
    for n in values:
        sign, words = n.to_sign_words()
        assert IBig.from_sign_words(sign, words) == n
        assert IBig.from_sign_words(sign, list(words)) == n

    # zero has a positive sign, and the sign of zero is normalized on construction
    sign, words = IBig(0).to_sign_words()
    assert sign == PySign.Positive and len(words) == 0
    assert IBig.from_sign_words(PySign.Negative, words) == IBig(0)
    assert IBig.from_sign_words(PySign.Positive, [0, 0]) == IBig(0)

    # leading zero words in the magnitude are trimmed
    assert IBig.from_sign_words(PySign.Negative, [1, 0]) == IBig(-1)

if __name__ == "__main__":
    test_words_get()
    test_words_set()
    test_words_del()
    test_words_sign_roundtrip()
