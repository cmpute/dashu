from dashu import UBig, experimental
from timeit import timeit
from random import randint, seed

def concrete_ntt_mul(a: UBig, b: UBig):
    base_size = 29

    a_chunks = [int(c) for c in a.to_chunks(base_size)]
    b_chunks = [int(c) for c in b.to_chunks(base_size)]
    
    fft_size = len(a_chunks) + len(b_chunks)
    fft_size = 2**fft_size.bit_length()
    a_chunks += [0] * (fft_size - len(a_chunks))
    b_chunks += [0] * (fft_size - len(b_chunks))
    if len(a) < 10000:
        plan = experimental.NttPlan64(fft_size, experimental.P_SOLINAS)
    else:
        plan = experimental.NttPlan64(fft_size)

    prod = plan.polymul(a_chunks, b_chunks)
    return UBig.from_chunks([c for c in prod], base_size)

def mul_test(a: int, b: int):
    N = 10**6 // a.bit_length()
    native_mul = timeit("(a * b) & 1", globals=dict(a=a, b=b), number=N) / N
    dashu_mul = timeit("(a * b) & 1", globals=dict(a=UBig(a), b=UBig(b)), number=N) / N
    concrete_ntt_time = timeit("ntt_mul(a, b) & 1", globals=dict(a=UBig(a), b=UBig(b), ntt_mul=concrete_ntt_mul), number=N) / N

    dashu_ratio = dashu_mul / native_mul
    concrete_ntt_ratio = concrete_ntt_time / native_mul

    print(f"native: {native_mul:.3e}, dashu: {dashu_mul:.3e} ({dashu_ratio:.2f}x), concrete-ntt: {concrete_ntt_time:.3e} ({concrete_ntt_ratio:.2f}x)")

def main():
    test_bits = [i * 10**j for j in range(2, 6) for i in [2, 5, 10]]
    for bits in test_bits:
        print(f"Testing {bits} bits")
        a = randint(1 << (bits // 2), 1 << bits)
        b = randint(1 << (bits // 2), 1 << bits)

        # assert concrete_ntt_mul(UBig(a), UBig(b)) == a * b
        mul_test(a, b)

if __name__ == "__main__":
    main()
