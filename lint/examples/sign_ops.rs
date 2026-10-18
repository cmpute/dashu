#![allow(dead_code)]
use dashu_base::Sign::{self, Negative, Positive};

const fn const_sign(negative: bool) -> Sign {
    // Not flagged: From<bool> is not const.
    if negative { Negative } else { Positive }
}

fn main() {
    let a = std::hint::black_box(Negative);
    let b = std::hint::black_box(Positive);
    let flag = std::hint::black_box(true);
    let _ = if flag { Negative } else { Positive };
    let _ = if flag { Positive } else { Negative };
    let _ = if a == b { Positive } else { Negative };
    let _ = if a != b { Negative } else { Positive };
    let _ = if a == b { Negative } else { Positive };
    let _ = if a != b { Positive } else { Negative };
    let _ = if a == Negative { Negative } else { Positive };
    let _ = if Negative != a { Positive } else { Negative };
    let _ = if a == Positive { Negative } else { Positive };
    let _ = if Positive != a { Negative } else { Positive };
    // Not flagged: no conversion, side effects in the branch, or another enum.
    let _ = if flag { Negative } else { Negative };
    let _ = if flag {
        std::hint::black_box(());
        Negative
    } else {
        Positive
    };
    enum Other {
        Positive,
        Negative,
    }
    let _ = if flag {
        Other::Negative
    } else {
        Other::Positive
    };
    #[allow(manual_sign_from_bool)]
    let _ = if flag { Negative } else { Positive };
    let _ = const_sign(flag);
}
