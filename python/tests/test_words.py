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

def test_index_out_of_index_range():
    # an integer index that does not fit in an index-sized integer is an IndexError, as for
    # a list and as for any other out-of-range index (it used to be a TypeError)
    import pytest
    for index in (2, -2, 2**63 - 1, 2**63, -2**63 - 1, 2**64):
        with pytest.raises(IndexError):
            Words([1])[index]
        with pytest.raises(IndexError):
            Words([1])[index] = 0
        with pytest.raises(IndexError):
            del Words([1])[index]
    with pytest.raises(TypeError):
        Words([1])["a"]
    assert Words([1, 2])[-1] == 2


def test_words_repeat():
    assert list(Words([1, 2]) * 2) == [1, 2, 1, 2]
    assert list(Words([1, 2]) * 0) == list(Words([1, 2]) * -3) == []
    assert list(Words([]) * 2**62) == []


def test_words_repeat_too_large():
    # a repetition that cannot be allocated raises MemoryError, as for a list: it panicked
    # ("capacity overflow") or aborted the interpreter on the failed allocation. Each case runs
    # in its own interpreter, since an abort cannot be caught.
    import subprocess
    import sys
    code = (
        "from dashu import Words\n"
        "try:\n"
        "    {}\n"
        "    print('returned')\n"
        "except MemoryError:\n"
        "    print('MemoryError')\n"
    )
    # (2**58 words do not fit in any address space, and the byte length of the others
    # overflows; a count that the system might try to serve is deliberately not tested)
    for expr in ("Words([1]) * 2**58", "Words([1]) * 2**62", "Words([1, 2, 3]) * (2**63 - 1)"):
        run = subprocess.run(
            [sys.executable, "-c", code.format(expr)], capture_output=True, text=True, timeout=60
        )
        assert run.returncode == 0, (expr, run.returncode, run.stderr)
        assert run.stdout.strip() == "MemoryError", (expr, run.stdout, run.stderr)


if __name__ == "__main__":
    test_words_get()
    test_words_set()
    test_words_del()
    test_words_sign_roundtrip()
