//! Differential oracle for add/sub: compare Context::add/sub against the exact sum
//! (precision 0) rounded to p OR p+1 (guard-digit tolerant), across modes/bases/precisions.
//! Run: cargo run -p dashu-float --release --example add_sub_oracle
use dashu_float::round::mode::{Away, Down, HalfAway, HalfEven, Up, Zero};
use dashu_float::round::Round;
use dashu_float::{Context, FBig, Repr};
use dashu_int::{IBig, UBig, Word};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn usize(&mut self, n: usize) -> usize {
        (self.next() as usize) % n
    }
    fn isize_in(&mut self, lo: isize, hi: isize) -> isize {
        lo + (self.next() as isize).rem_euclid(hi - lo + 1)
    }
    fn sig(&mut self, base: Word, digits: usize) -> IBig {
        let mut mag = UBig::from_word(0);
        let place = UBig::from_word(base);
        for i in 0..digits {
            mag += UBig::from_word(self.next() % base) * place.pow(i);
        }
        if mag.is_zero() {
            mag = UBig::from_word(1);
        }
        IBig::from(if self.next() & 1 == 0 { 1 } else { -1 }) * IBig::from(mag)
    }
}

fn oracle<R: Round, const B: Word>(
    a: &Repr<B>,
    b: &Repr<B>,
    sub: bool,
    p: usize,
) -> (Repr<B>, Repr<B>) {
    let e = Context::<R>::new(0);
    let exact = if sub { e.sub(a, b) } else { e.add(a, b) };
    let er = exact.value().repr().clone();
    let d = er.digits().max(p + 2).max(1);
    let rp = FBig::<R, B>::from_repr(er.clone(), Context::<R>::new(d))
        .with_precision(p)
        .value()
        .repr()
        .clone();
    let rp1 = FBig::<R, B>::from_repr(er, Context::<R>::new(d))
        .with_precision(p + 1)
        .value()
        .repr()
        .clone();
    (rp, rp1)
}

fn run<R: Round, const B: Word>(
    rng: &mut Rng,
    mode: &str,
    base: Word,
    precisions: &[usize],
) -> usize {
    let mut bad = 0usize;
    for &p in precisions {
        let ctx = Context::<R>::new(p);
        for _ in 0..6000 {
            let da = 1 + rng.usize(10);
            let db = 1 + rng.usize(10);
            let a = Repr::<B>::new(rng.sig(base, da), rng.isize_in(-8, 8));
            let b = Repr::<B>::new(rng.sig(base, db), rng.isize_in(-8, 8));
            if a.is_zero() || b.is_zero() || !a.is_finite() || !b.is_finite() {
                continue;
            }
            for sub in [false, true] {
                let got = if sub {
                    ctx.sub(&a, &b)
                } else {
                    ctx.add(&a, &b)
                }
                .value()
                .repr()
                .clone();
                let (rp, rp1) = oracle::<R, B>(&a, &b, sub, p);
                if got != rp && got != rp1 {
                    bad += 1;
                    if bad <= 5 {
                        eprintln!("MISMATCH [{mode}, B={base}, p={p}, sub={sub}] a={a:?} b={b:?} got={got:?} rp={rp:?} rp1={rp1:?}");
                    }
                }
            }
        }
    }
    bad
}

fn main() {
    let mut rng = Rng(0x123456789abcdef0);
    let mut total = 0usize;
    macro_rules! sweep {
        ($m:ident, $b:literal) => {{
            let n = run::<$m, $b>(&mut rng, stringify!($m), $b, &[1, 2, 3, 5, 10, 50, 200]);
            println!("{} base {}: {} mismatches", stringify!($m), $b, n);
            total += n;
        }};
    }
    sweep!(Zero, 2);
    sweep!(Away, 2);
    sweep!(Up, 2);
    sweep!(Down, 2);
    sweep!(HalfEven, 2);
    sweep!(HalfAway, 2);
    sweep!(Zero, 10);
    sweep!(Away, 10);
    sweep!(Up, 10);
    sweep!(Down, 10);
    sweep!(HalfEven, 10);
    sweep!(HalfAway, 10);
    println!("TOTAL: {total}");
    if total > 0 {
        std::process::exit(1);
    }
}
