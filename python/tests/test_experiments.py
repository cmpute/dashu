from dashu import UBig, experimental

def test_concrete_ntt():
    a = UBig(3**2000)
    b = UBig(7**1100)

    # ----- native64 test -----
    base_size = 29
    fft_size = 256 # must be power of two

    a_chunks = [int(c) for c in a.to_chunks(base_size)]
    b_chunks = [int(c) for c in b.to_chunks(base_size)]
    # print("original_size", len(a_chunks), len(b_chunks))
    a_chunks += [0] * (fft_size - len(a_chunks))
    b_chunks += [0] * (fft_size - len(b_chunks))

    plan = experimental.NttPlan64(fft_size)

    a_fft = plan.fwd(a_chunks)
    a_recon = UBig.from_chunks([c // fft_size for c in plan.inv(a_fft)], base_size)
    assert a == a_recon
    b_fft = plan.fwd(b_chunks)
    b_recon = UBig.from_chunks([c // fft_size for c in plan.inv(b_fft)], base_size)
    assert b == b_recon

    prod = plan.polymul(a_chunks, b_chunks)
    prod_recon = UBig.from_chunks([c for c in prod], base_size)
    assert a * b == prod_recon

    # ----- prime64 test -----
    base_size = 29
    fft_size = 256 # must be power of two

    a_chunks = [int(c) for c in a.to_chunks(base_size)]
    b_chunks = [int(c) for c in b.to_chunks(base_size)]
    # print("original_size", len(a_chunks), len(b_chunks))
    a_chunks += [0] * (fft_size - len(a_chunks))
    b_chunks += [0] * (fft_size - len(b_chunks))
    
    plan = experimental.NttPlan64(fft_size, experimental.P_SOLINAS)
    
    a_fft = plan.fwd(a_chunks)
    a_recon = UBig.from_chunks([c // fft_size for c in plan.inv(a_fft)], base_size)
    assert a == a_recon
    b_fft = plan.fwd(b_chunks)
    b_recon = UBig.from_chunks([c // fft_size for c in plan.inv(b_fft)], base_size)
    assert b == b_recon

    prod = plan.polymul(a_chunks, b_chunks)
    prod_recon = UBig.from_chunks([c for c in prod], base_size)
    assert a * b == prod_recon


if __name__ == "__main__":
    test_concrete_ntt()
