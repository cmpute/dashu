#!/usr/bin/env python3
"""Mandelbrot set rendered with arbitrary-precision arithmetic from dashu.

Each pixel iterates the map ``z <- z² + c`` until ``|z|² > 4``. dashu's :class:`~dashu.FBig`
(base-2 arbitrary-precision float) carries the orbit, so the only limit on zoom depth is
patience — there is no hidden ``f64`` floor the way there is with Python's built-in
``complex``.

The default view is "Shades of Gray" from the silversky gallery. Pass ``--width`` to zoom
in; the script raises the working precision
automatically and reports where ``f64`` would run out of resolution (around a view width
of ``~3e-16`` near the unit circle). Beyond that floor, native ``complex`` arithmetic
collapses neighbouring pixels to the same value and the render turns to banding, while
dashu keeps every pixel distinct.

Pass ``--mpmath`` to render an additional panel with `mpmath`'s ``mpf`` at the *same*
binary precision — an independent reference to check the dashu output against. In smooth
regions all three backends agree exactly; near the fractal boundary they are sensitive to
last-bit differences (dashu's base-2 `FBig` rounds toward zero, `f64`/`mpmath` round to
nearest), so a little speckle there is normal.

Examples::

    python mandelbrot_zoom.py                       # default Seahorse Valley view
    python mandelbrot_zoom.py --width 1e-6          # deeper zoom, more spirals
    python mandelbrot_zoom.py --width 1e-20         # far past f64's floor (slow)
    python mandelbrot_zoom.py --cbig                # iterate with CBig (canonical, slower)
    python mandelbrot_zoom.py --width 3 --center=-0.5,0   # the classic full set
    python mandelbrot_zoom.py --compare --mpmath    # dashu vs f64 vs mpmath reference
    python mandelbrot_zoom.py --deep --compare      # self-similar point where f64 collapses

Galleries: https://mandelbrot.silversky.dev/gallery/
"""

import argparse
import math
import sys

from dashu import CBig, DBig, FBig

# Optional reference backend. mpmath's `workprec(n)` takes bits, so it can run at the exact
# same binary precision as dashu's FBig — two independent implementations at matched
# precision, where agreement validates the dashu render.
try:
    import mpmath
    HAVE_MPMATH = True
except ImportError:
    HAVE_MPMATH = False

# "Shades of Gray" — a location from the silversky gallery (mandelbrot.silversky.dev).
DEFAULT_CENTER = ("-0.7501112710122269", "-0.009161334147374207")
DEFAULT_WIDTH = 1e-2

# Deep self-similar Misiurewicz point M(3,4). Its filament structure survives to ~1e-16,
# where f64's coordinates collapse to a single value (native `complex` renders uniform
# garbage) while dashu renders the true spiral. Pair `--deep` with `--compare` to see it.
DEEP_CENTER = ("-1.14436587739903192261971", "0.314719470372146774965374")
DEEP_WIDTH = 1e-16
# Escapes at this view top out ~134, so a tight cap keeps the √-gamma shading from crushing
# the whole structure into one band (a too-large max_iter makes every pixel the same char).
DEEP_MAX_ITER = 400

# Exterior is shaded by escape speed (fast escape = space, slow = dense); the interior of
# the set is a solid block. A sqrt gamma curve spreads the common fast-escape pixels across
# the whole ramp instead of crushing them into one band.
PALETTE = " .,:;ox%#@"  # fast escape (space) -> slow escape (dense)
INTERIOR = "█"          # inside the set


def shade_char(n, max_iter):
    """ASCII cell for an orbit that escaped at iteration `n` (or `max_iter` if bounded)."""
    if n >= max_iter:
        return INTERIOR
    t = math.sqrt(n / max_iter)
    return PALETTE[min(len(PALETTE) - 1, int(t * len(PALETTE)))]


def required_prec(width, cols, center_mag):
    """Binary bits so pixel-sized steps around `center_mag` stay distinct in FBig."""
    pixel = width / cols
    rel = pixel / max(abs(center_mag), pixel)
    return max(53, math.ceil(-math.log2(rel)) + 16)  # 16 guard bits over the resolution budget


def to_fbig(decimal_string, prec):
    """Parse an exact decimal string into a base-2 FBig at `prec` bits.

    ``DBig(string)`` retains only the literal's significant digits, so a direct
    ``to_binary()`` would lose precision — e.g. ``"-0.7453"`` rounds to ~13 bits
    (``-0.745285…``) instead of ``-0.7453``. At a deep zoom that base-point error dwarfs
    the whole view and FBig ends up iterating the wrong point. Raise the decimal precision
    first so the base-10 -> base-2 conversion is correctly rounded at ``prec`` bits.
    """
    ddigits = int(prec / math.log2(10)) + 4
    return DBig(decimal_string).with_precision(ddigits).to_binary().with_precision(prec)


def _fbig_escape(cre, cim, max_iter, zero, four):
    """Escape time of ``z <- z² + c`` with `cre`/`cim` as real FBig parts (scalar kernel)."""
    zr, zi = zero, zero
    n = 0
    while n < max_iter:
        zr2 = zr * zr
        zi2 = zi * zi
        if zr2 + zi2 > four:
            break
        zi = (zr + zi) * (zr + zi) - zr2 - zi2 + cim
        zr = zr2 - zi2 + cre
        n += 1
    return n


def render_fbig(center_re, center_im, width, cols, rows, max_iter, prec):
    """Scalar FBig iteration (z and c tracked as separate real floats) — the fast path.

    This is the standard decomposition of ``z² + c``; it uses only real FBig arithmetic,
    which is several times faster than building a CBig per step. Returns
    ``(image, stats)`` where ``stats`` tallies every pixel's escape time so the caller can
    classify the view exactly.
    """
    def part(decimal_string):
        return to_fbig(decimal_string, prec)

    def offset(value):
        return FBig(value).with_precision(prec)  # offsets are small; float input is precise enough

    zero = FBig(0).with_precision(prec)
    four = FBig(4).with_precision(prec)
    height = width * rows / cols * 2.0  # terminal characters are ~2:1

    lines, escaping, bounded = [], [], 0
    for py in range(rows):
        cim = part(center_im) + offset((py / (rows - 1) - 0.5) * height)
        chars = []
        for px in range(cols):
            cre = part(center_re) + offset((px / (cols - 1) - 0.5) * width)
            n = _fbig_escape(cre, cim, max_iter, zero, four)
            if n >= max_iter:
                bounded += 1
            else:
                escaping.append(n)
            chars.append(shade_char(n, max_iter))
        lines.append("".join(chars))
    stats = {"total": cols * rows, "bounded": bounded, "escaping": escaping}
    return "\n".join(lines), stats


def render_mpmath(center_re, center_im, width, cols, rows, max_iter, prec):
    """Reference render with `mpmath` (`mpf`) at the same binary precision.

    Independent of dashu, so it arbitrates the dashu-vs-native-``complex`` comparison:
    whichever panel it matches is the correct one. ``mpmath.workprec`` takes bits, matching
    the FBig precision exactly.
    """
    if not HAVE_MPMATH:
        raise RuntimeError("mpmath is not installed")
    height = width * rows / cols * 2.0
    with mpmath.workprec(prec):
        cre0 = mpmath.mpf(center_re)
        cim0 = mpmath.mpf(center_im)
        lines = []
        for py in range(rows):
            cim = cim0 + mpmath.mpf((py / (rows - 1) - 0.5) * height)
            chars = []
            for px in range(cols):
                cre = cre0 + mpmath.mpf((px / (cols - 1) - 0.5) * width)
                zr, zi = mpmath.mpf(0), mpmath.mpf(0)
                n = 0
                while n < max_iter:
                    zr2 = zr * zr
                    zi2 = zi * zi
                    if zr2 + zi2 > 4:
                        break
                    zi = (zr + zi) * (zr + zi) - zr2 - zi2 + cim
                    zr = zr2 - zi2 + cre
                    n += 1
                chars.append(shade_char(n, max_iter))
            lines.append("".join(chars))
    return "\n".join(lines)


def render_cbig(center_re, center_im, width, cols, rows, max_iter, prec):
    """Canonical CBig iteration — ``z`` and ``c`` as complex numbers. Slower but direct.

    Returns ``(image, stats)`` (see :func:`render_fbig`).
    """
    def coord(base, frac, span):
        return to_fbig(base, prec) + FBig(frac * span).with_precision(prec)

    zero = FBig(0).with_precision(prec)
    height = width * rows / cols * 2.0

    lines, escaping, bounded = [], [], 0
    for py in range(rows):
        cim = coord(center_im, py / (rows - 1) - 0.5, height)
        chars = []
        for px in range(cols):
            cre = coord(center_re, px / (cols - 1) - 0.5, width)
            c = CBig.from_parts(cre, cim)
            z = CBig.from_parts(zero, zero)
            n = 0
            while n < max_iter and z.norm() <= 4:
                z = z * z + c
                n += 1
            if n >= max_iter:
                bounded += 1
            else:
                escaping.append(n)
            chars.append(shade_char(n, max_iter))
        lines.append("".join(chars))
    stats = {"total": cols * rows, "bounded": bounded, "escaping": escaping}
    return "\n".join(lines), stats


def render_float(center_re, center_im, width, cols, rows, max_iter):
    """Same view with native ``complex`` (f64). Bands/collapses once pixels out-resolve f64."""
    cre0, cim0 = float(center_re), float(center_im)
    height = width * rows / cols * 2.0
    lines = []
    for py in range(rows):
        cim = cim0 + (py / (rows - 1) - 0.5) * height
        chars = []
        for px in range(cols):
            cre = cre0 + (px / (cols - 1) - 0.5) * width
            c = complex(cre, cim)
            z = 0j
            n = 0
            while n < max_iter and z.real * z.real + z.imag * z.imag <= 4:
                z = z * z + c
                n += 1
            chars.append(shade_char(n, max_iter))
        lines.append("".join(chars))
    return "\n".join(lines)


def precision_report(center_re, width, cols, prec):
    """Print where f64 loses resolution versus dashu at this zoom."""
    step = width / cols
    f_center = float(center_re)
    ulp = abs(f_center) * 2.0 ** -52 if f_center else 2.0 ** -1074
    distinct = float(center_re) != float(DBig(center_re) + DBig(step))
    floor = ulp * cols  # view width at which one pixel ≈ one f64 ULP
    status = "f64 resolves pixels here" if distinct else "f64 is OUT OF RESOLUTION"
    print(f"  view width          : {width:.3e}")
    print(f"  pixel step          : {step:.3e}")
    print(f"  f64 ULP at centre   : {ulp:.3e}   (f64 floor ≈ width {floor:.1e})")
    print(f"  {status}; dashu runs at {prec} bits.")


def structure_verdict(total, bounded, escaping, max_iter):
    """One-line classification: does this view actually contain fractal detail?

    Based on the exact per-pixel escape times from a full render (not a sub-sample), so
    even thin boundary filaments are counted.
    """
    if bounded == total:
        return ("no pixel escaped within max_iter — interior, "
                "or --max-iter too low (try raising it)")
    if not escaping:
        return "all bounded"
    distinct = len(set(escaping))
    lo, hi = min(escaping), max(escaping)
    if distinct <= 3:
        return (f"uniform exterior — every pixel escapes ~{lo} "
                "(smooth region; no detail at this zoom)")
    pct = f", {bounded / total:.0%} bounded" if bounded else ""
    return (f"structured — escape times span {lo}..{hi} "
            f"({distinct} distinct values over {total} pixels{pct})")


def parse_center(text):
    re_s, im_s = text.split(",")
    return re_s.strip(), im_s.strip()


def main():
    ap = argparse.ArgumentParser(description="Mandelbrot set via dashu arbitrary-precision floats.")
    ap.add_argument("--center", type=parse_center, default=DEFAULT_CENTER,
                    help="'re,im' decimal strings (default: Shades of Gray)")
    ap.add_argument("--width", type=float, default=DEFAULT_WIDTH,
                    help="view width in the complex plane (default: %(default)s)")
    ap.add_argument("--cols", type=int, default=70)
    ap.add_argument("--rows", type=int, default=26)
    ap.add_argument("--max-iter", type=int, default=500)
    ap.add_argument("--prec", type=int, default=0,
                    help="binary precision in bits (default: auto from --width)")
    ap.add_argument("--cbig", action="store_true",
                    help="iterate with CBig (canonical complex type) instead of scalar FBig")
    ap.add_argument("--compare", action="store_true",
                    help="also render the view with native complex (f64)")
    ap.add_argument("--mpmath", action="store_true",
                    help="also render an mpmath (mpf) reference panel at the same precision")
    ap.add_argument("--deep", action="store_true",
                    help="preset: deep self-similar Misiurewicz point at 1e-16 where f64 "
                         "collapses (ignores --center/--width/--max-iter; pair with --compare "
                         "to see native complex fail)")
    args = ap.parse_args()

    # --deep is a preset for the location/zoom/iters; it ignores --center/--width/--max-iter.
    if args.deep:
        center_re, center_im = DEEP_CENTER
        width, max_iter = DEEP_WIDTH, DEEP_MAX_ITER
    else:
        center_re, center_im = args.center
        width, max_iter = args.width, args.max_iter

    prec = args.prec or required_prec(width, args.cols, abs(float(center_re)))
    kind = "CBig" if args.cbig else "FBig (scalar)"
    interactive = sys.stdout.isatty()

    # Print the config header up front so it's visible while a deep render runs. The
    # structure verdict needs every pixel's escape time, so it is filled in just above the
    # image once the render finishes. In a terminal a "rendering…" placeholder is shown and
    # then overwritten; when piped, no placeholder is emitted so logs stay clean.
    print("Mandelbrot set with dashu arbitrary-precision arithmetic")
    print("=" * 50)
    print(f"  centre      : {center_re} + {center_im}i")
    print(f"  grid / iters: {args.cols}x{args.rows}, max {max_iter}")
    print(f"  iteration   : {kind} at {prec} bits")
    precision_report(center_re, width, args.cols, prec)
    if interactive:
        print("  structure   : rendering…", flush=True)
    else:
        sys.stdout.flush()

    renderer = render_cbig if args.cbig else render_fbig
    image, stats = renderer(center_re, center_im, width, args.cols, args.rows, max_iter, prec)

    verdict = structure_verdict(stats["total"], stats["bounded"], stats["escaping"], max_iter)
    line = f"  structure   : {verdict}"
    if interactive:
        print(f"\r\x1b[2K{line}")  # clear the placeholder line, then print the verdict
    else:
        print(line)

    print(f"\n--- dashu {kind} ---")
    print(image)

    if args.mpmath:
        if not HAVE_MPMATH:
            print("\n(mpmath not installed — `pip install mpmath` for the reference panel)")
        else:
            print("\n--- mpmath reference (mpf, same precision) ---")
            print(render_mpmath(center_re, center_im, width, args.cols, args.rows, max_iter, prec))

    if args.compare:
        print("\n--- native complex (f64) ---")
        print(render_float(center_re, center_im, width, args.cols, args.rows, max_iter))


if __name__ == "__main__":
    main()
