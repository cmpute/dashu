//! Suggestions retain clones and use right-hand sign multiplication.

use dashu_base::Sign;
use dashu_int::IBig;
use dashu_ratio::RBig;

fn main() {
    let sign = std::hint::black_box(Sign::Negative);
    let hi = std::hint::black_box(IBig::from(-5));

    // Flagged: preserve the clone, including when hi is used after the branch.
    let hi_signed = if sign == Sign::Negative {
        -hi.clone()
    } else {
        hi.clone()
    };

    // Flagged: `!=` with `Sign::Positive`.
    let _ = if sign != Sign::Positive {
        -hi.clone()
    } else {
        hi.clone()
    };

    // Flagged: the `Sign::Positive` mirror.
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

    // RBig implements RBig * Sign, so the suggestion's operand order matters.
    let ratio = RBig::from(-7);
    let _ = if Sign::Negative == sign {
        -ratio.clone()
    } else {
        ratio.clone()
    };
    let _ = if Sign::Positive != sign {
        -ratio.clone()
    } else {
        ratio.clone()
    };
    let _ = if Sign::Positive == sign {
        ratio.clone()
    } else {
        -ratio.clone()
    };
    let _ = if Sign::Negative != sign {
        ratio.clone()
    } else {
        -ratio.clone()
    };
    let _ = if sign == Sign::Negative {
        -ratio
    } else {
        ratio
    };

    // Not flagged: arbitrary Neg types need not implement multiplication by Sign.
    struct Other;
    impl std::ops::Neg for Other {
        type Output = Self;
        fn neg(self) -> Self {
            self
        }
    }
    let other = Other;
    let _ = if sign == Sign::Negative {
        -other
    } else {
        other
    };

    // Not flagged: the branch includes a side effect.
    let value = IBig::ONE;
    let _ = if sign == Sign::Negative {
        std::hint::black_box(());
        -value.clone()
    } else {
        value
    };
}
