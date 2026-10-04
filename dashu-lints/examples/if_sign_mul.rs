//! Examples for the `if_sign_mul` lint: each `flagged` block should produce a warning whose
//! suggestion is `sign * hi`.

use dashu_base::Sign;
use dashu_int::IBig;

fn main() {
    let sign = std::hint::black_box(Sign::Negative);
    let hi = std::hint::black_box(IBig::from(-5));

    // flagged: the issue's pattern; the suggestion is `sign * hi`, marked `MaybeIncorrect`
    // because it drops the clones.
    let hi_signed = if sign == Sign::Negative {
        -hi.clone()
    } else {
        hi.clone()
    };

    // flagged: `!=` with `Sign::Positive`, `MachineApplicable`.
    let _ = if sign != Sign::Positive {
        -hi.clone()
    } else {
        hi.clone()
    };

    // flagged: the `Sign::Positive` mirror, `MachineApplicable`.
    let _ = if sign == Sign::Positive {
        hi.clone()
    } else {
        -hi.clone()
    };

    // not flagged: the attribute silences the lint.
    #[allow(if_sign_mul)]
    let _ = if sign == Sign::Negative {
        hi.clone()
    } else {
        -hi.clone()
    };

    // not flagged: the branches are not a value and its negation.
    let _ = if sign == Sign::Negative {
        hi.clone()
    } else {
        IBig::ONE
    };

    // The suggested form: fine (moves `hi`; nothing uses it afterwards).
    let _ = sign * hi;

    std::hint::black_box((hi_signed, sign));
}
