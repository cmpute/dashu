# dashu-rs

## What is dashu?

[dashu](https://github.com/cmpute/dashu) is an arbitrary-precision number library
written in pure Rust, aiming to be a Rust-native alternative to GNU GMP + MPFR. It
provides unlimited-precision integers, rationals, floats, and complex numbers with
correct rounding, split across a set of Rust crates (`dashu-int`, `dashu-float`,
`dashu-ratio`, `dashu-cmplx`, …).

## What is dashu-rs?

`dashu-rs` is a **standalone Python package** that brings dashu's number types to
Python. It is a native extension built with [PyO3](https://pyo3.rs), distributed as
a prebuilt wheel on [PyPI](https://pypi.org/project/dashu-rs/), and **installs
anywhere with `pip`** — no Rust toolchain, no compiler, and no knowledge of the
dashu Rust crates required:

```sh
pip install dashu-rs
```

```python
from dashu import UBig, IBig, RBig, FBig, DBig, CBig
```

The exposed types — arbitrary-precision integers (`UBig`/`IBig`), rationals
(`RBig`), binary/decimal floats (`FBig`/`DBig`), and complex numbers (`CBig`) —
are `Send` + `Sync` (free-threaded-Python compatible), accept native Python
numbers in their constructors, and interoperate with `int`/`float`/`Fraction` in
arithmetic.

## Usage

See **[USAGE.md](./USAGE.md)** for the full guide — installation, constructors,
operations, formatting, transcendentals, and the module-level `math` API.

It is also rendered in the dashu user guide at
<https://zyxin.xyz/dashu/python.html>.
