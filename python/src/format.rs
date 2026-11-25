//! Python format mini-language (`__format__`) support.
//!
//! Integers (`UBig`/`IBig`) delegate to Python's own `int.__format__` (Python ints are
//! arbitrary precision, so there is no loss). Floats (`FBig`/`DBig`) are rendered in decimal
//! (an `FBig` is first converted to base 10) using dashu's precision-aware formatting, then
//! the Python spec's layout (sign / width / align / fill / zero-pad / grouping) and the
//! scientific exponent are normalized to CPython conventions.

use dashu::base::Sign;
use dashu::float::{Context, DBig, FBig, Repr};
use dashu::integer::UBig;
use pyo3::PyResult;
use pyo3::exceptions::PyValueError;

/// A parsed Python format spec: `[[fill]align][sign][#][0][width][grouping][.precision][type]`.
#[allow(dead_code)]
#[derive(Clone)]
pub struct Spec {
    pub fill: char,
    pub align: Option<char>, // '<' '>' '=' '^'
    pub sign: char,          // '+' '-' ' '
    pub alt: bool,           // '#'
    pub zero: bool,          // '0' (zero-pad)
    pub width: Option<usize>,
    pub group: Option<char>, // ',' or '_'
    pub prec: Option<usize>,
    pub ty: char, // '\0' = none
}

const FLOAT_TYPES: &str = "eEfFgGn%";

pub fn parse(spec: &str) -> PyResult<Spec> {
    let chars: Vec<char> = spec.chars().collect();
    let mut i = 0;
    let n = chars.len();

    // [[fill]align]
    let mut fill = ' ';
    let mut fill_given = false;
    let mut align = None;
    if n >= 2 && "<>=^".contains(chars[1]) {
        fill = chars[0];
        fill_given = true;
        align = Some(chars[1]);
        i = 2;
    } else if n >= 1 && "<>=^".contains(chars[0]) {
        align = Some(chars[0]);
        i = 1;
    }

    // [sign]
    let mut sign = '-';
    if i < n && "+- ".contains(chars[i]) {
        sign = chars[i];
        i += 1;
    }

    // [#]
    let mut alt = false;
    if i < n && chars[i] == '#' {
        alt = true;
        i += 1;
    }

    // [0]: the zero-padding flag, unless a fill character was given explicitly: then, as in
    // CPython, the `0` is only the first digit of the width (`x<010` pads with `x` to 10)
    let mut zero = false;
    if i < n && chars[i] == '0' {
        zero = !fill_given;
        i += 1;
    }

    // [width]
    let mut width = None;
    let start = i;
    while i < n && chars[i].is_ascii_digit() {
        i += 1;
    }
    if i > start {
        width = Some(chars[start..i].iter().collect::<String>().parse().unwrap());
    }

    // [grouping]
    let mut group = None;
    if i < n && (chars[i] == ',' || chars[i] == '_') {
        group = Some(chars[i]);
        i += 1;
    }

    // [.precision]
    let mut prec = None;
    if i < n && chars[i] == '.' {
        i += 1;
        let start = i;
        while i < n && chars[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return Err(PyValueError::new_err("missing precision in format spec"));
        }
        prec = Some(chars[start..i].iter().collect::<String>().parse().unwrap());
    }

    // [type]
    let ty = if i < n {
        let t = chars[i];
        if i + 1 != n {
            return Err(PyValueError::new_err(format!("invalid format specifier '{spec}'")));
        }
        t
    } else {
        '\0'
    };

    if alt {
        // The '#' alternate-form flag is parsed but not yet implemented for float formatting;
        // reject it explicitly rather than silently ignoring it (see dashu-python CHANGELOG).
        return Err(PyValueError::new_err(
            "the '#' (alternate form) flag is not supported for float formatting",
        ));
    }

    Ok(Spec {
        fill,
        align,
        sign,
        alt,
        zero,
        width,
        group,
        prec,
        ty,
    })
}

impl Spec {
    fn is_float_type(&self) -> bool {
        self.ty == '\0' || FLOAT_TYPES.contains(self.ty)
    }
}

/// Render a `DBig` (base-10 float) according to a parsed Python spec, including layout.
pub fn format_dbig(d: &DBig, spec_str: &str) -> PyResult<String> {
    format_dbig_with(d, &parse(spec_str)?)
}

/// Render a `DBig` according to an already parsed spec (see [`format_dbig`]).
fn format_dbig_with(d: &DBig, s: &Spec) -> PyResult<String> {
    if !s.is_float_type() {
        let ty_str = if s.ty == '\0' {
            String::new()
        } else {
            s.ty.to_string()
        };
        return Err(PyValueError::new_err(format!(
            "unknown format code '{ty_str}' for object of type 'float'"
        )));
    }

    if d.repr().is_infinite() {
        // no digits to round, group or zero-pad
        let body = if matches!(s.ty, 'E' | 'F' | 'G') {
            "INF"
        } else {
            "inf"
        };
        return Ok(apply_layout(d.repr().sign() == Sign::Negative, "", body, false, s));
    }

    // Produce the unsigned (no sign policy) numeric body in decimal.
    let (negative, body) = render_body(d, s)?;
    Ok(apply_layout(negative, "", &body, true, s))
}

/// Render a base-2 `FBig` in one of the decimal presentation types.
///
/// A binary float is always exactly representable in decimal. Wherever the spec fixes the
/// digits to print (an explicit precision, or the fixed notations with their default of 6
/// fraction digits), they are the digits of the exact value, correctly rounded once by the
/// formatting. (Converting with `to_decimal()` first would round to the ~`0.3 * precision`
/// decimal digits that the binary precision is worth, and print zeros after them.)
///
/// Where the spec leaves the number of digits open (`'e'` / `'g'` without a precision), the
/// float is shown with the decimal digits that its binary precision is worth, as `to_decimal()`
/// gives them.
pub fn format_fbig_decimal(f: &FBig, spec_str: &str) -> PyResult<String> {
    format_fbig_decimal_with(f, &parse(spec_str)?)
}

/// Render a base-2 `FBig` in decimal according to an already parsed spec (see
/// [`format_fbig_decimal`]).
fn format_fbig_decimal_with(f: &FBig, s: &Spec) -> PyResult<String> {
    let exact = s.prec.is_some() || matches!(s.ty, 'f' | 'F' | '%');
    let d = if exact || f.repr().is_infinite() || f.precision() == 0 {
        fbig_to_decimal_exact(f)
    } else {
        f.to_decimal().value()
    };
    format_dbig_with(&d, s)
}

/// Render a complex number with the real part `re` and the imaginary part `im` as
/// `(re+imj)`, with both parts in one of the decimal presentation types.
///
/// As for Python's `complex`, the precision, the type and the grouping of the spec apply to
/// each part, the sign option to the real part (the imaginary part always has its sign), and
/// the width, fill and alignment once to the whole text. Zero-padding and the `=` alignment
/// have no meaning for the two numbers of a complex and are rejected.
pub fn format_complex(re: &FBig, im: &FBig, spec_str: &str) -> PyResult<String> {
    let s = parse(spec_str)?;
    if s.zero {
        return Err(PyValueError::new_err(
            "Zero padding is not allowed in complex format specifier",
        ));
    }
    if s.align == Some('=') {
        return Err(PyValueError::new_err(
            "'=' alignment flag is not allowed in complex format specifier",
        ));
    }

    let part = Spec {
        fill: ' ',
        align: None,
        width: None,
        ..s.clone()
    };
    let re = format_fbig_decimal_with(re, &part)?;
    let im = format_fbig_decimal_with(im, &Spec { sign: '+', ..part })?;
    let text = format!("({re}{im}j)");

    let pad = s.width.unwrap_or(0).saturating_sub(text.chars().count());
    let fill = |n: usize| std::iter::repeat_n(s.fill, n).collect::<String>();
    Ok(match s.align.unwrap_or('>') {
        '<' => format!("{text}{}", fill(pad)),
        '^' => format!("{}{text}{}", fill(pad / 2), fill(pad - pad / 2)),
        _ => format!("{}{text}", fill(pad)),
    })
}

/// The exact decimal expansion of a base-2 float (see [`format_fbig_decimal`]).
fn fbig_to_decimal_exact(f: &FBig) -> DBig {
    /// Beyond this exponent the exact expansion is impractically long; the rounded conversion
    /// is used instead.
    const MAX_EXACT_EXPONENT: usize = 1 << 20;

    let repr = f.repr();
    if repr.is_infinite() {
        return match repr.sign() {
            Sign::Positive => DBig::INFINITY,
            Sign::Negative => DBig::NEG_INFINITY,
        };
    }
    if repr.is_neg_zero() {
        return DBig::from_repr(Repr::neg_zero(), Context::new(1));
    }

    let exponent = repr.exponent();
    if exponent.unsigned_abs() > MAX_EXACT_EXPONENT && f.precision() != 0 {
        return f.to_decimal().value();
    }
    // m * 2^e = (m << e) for e >= 0, and (m * 5^-e) * 10^e for e < 0
    let exact = if exponent >= 0 {
        Repr::<10>::new(repr.significand() << exponent as usize, 0)
    } else {
        let scale = UBig::from(5u8).pow(exponent.unsigned_abs());
        Repr::<10>::new(repr.significand() * scale, exponent)
    };
    let precision = exact.digits().max(1);
    DBig::from_repr(exact, Context::new(precision))
}

/// Render the magnitude as a decimal string (without sign), per the spec's type.
fn render_body(d: &DBig, s: &Spec) -> PyResult<(bool, String)> {
    // When no precision is given, show the value's full native precision (its significant
    // digit count) rather than CPython's fixed default of 6 — this is an arbitrary-precision
    // library, so truncating to 6 digits by default would silently hide information.
    let native = d.digits();
    let (neg, raw) = match s.ty {
        'e' | 'E' => {
            // scientific: `native` significant digits => `native - 1` fraction digits
            let p = s.prec.unwrap_or(native.saturating_sub(1));
            let mut raw = format!("{:.*e}", p, d);
            raw = normalize_sci(raw, s.ty == 'E');
            strip_sign(&raw)
        }
        'f' | 'F' => {
            let p = s.prec.unwrap_or(6);
            strip_sign(&format!("{:.*}", p, d))
        }
        '%' => {
            let p = s.prec.unwrap_or(6);
            // ×100, fixed, then a trailing '%'
            let scaled = d.clone() * DBig::from(100u8);
            let (neg, mut body) = strip_sign(&format!("{:.*}", p, scaled));
            body.push('%');
            (neg, body)
        }
        'g' | 'G' | 'n' => render_general(d, s),
        '\0' => {
            // default: full plain decimal (dashu Display), with optional precision
            // limiting significant digits via a re-round.
            if let Some(p) = s.prec {
                let rounded = d.clone().with_precision(p.max(1)).value();
                strip_sign(&format!("{}", rounded))
            } else {
                strip_sign(&format!("{}", d))
            }
        }
        _ => unreachable!(),
    };
    Ok((neg, raw))
}

/// The general format (`'g'` / `'G'` / `'n'`) of Python: round to `p` significant digits, then
/// use the scientific notation if the exponent of the rounded value is below -4 or not below
/// `p`, and the fixed notation otherwise; insignificant trailing zeros are removed.
///
/// Where the spec has no precision, `p` is the precision of the value (rather than CPython's
/// fixed 6 for `float`), so no digit is hidden by default.
fn render_general(d: &DBig, s: &Spec) -> (bool, String) {
    let p = match s.prec {
        Some(p) => p.max(1),
        None => d.precision().max(d.digits()).max(1),
    };

    // the scientific rendering gives the exponent of the value rounded to `p` digits
    let sci = format!("{:.*e}", p - 1, d);
    let (neg, sci) = strip_sign(&sci);
    let (mantissa, exponent) = match sci.find(['e', 'E']) {
        Some(i) => (&sci[..i], sci[i + 1..].parse::<isize>().unwrap_or(0)),
        None => (sci.as_str(), 0),
    };

    let body = if exponent < -4 || exponent >= p as isize {
        let marker = if s.ty == 'G' { 'E' } else { 'e' };
        format!("{}{marker}{exponent:+03}", strip_trailing_zeros(mantissa))
    } else {
        let fraction_digits = (p as isize - 1 - exponent) as usize;
        let (_, fixed) = strip_sign(&format!("{:.*}", fraction_digits, d));
        strip_trailing_zeros(&fixed).to_string()
    };
    (neg, body)
}

/// Remove the trailing zeros of the fractional part (and the decimal point if nothing is left).
fn strip_trailing_zeros(digits: &str) -> &str {
    if digits.contains('.') {
        digits.trim_end_matches('0').trim_end_matches('.')
    } else {
        digits
    }
}

/// Render a base-2 `FBig` in hexadecimal (lossless: hex significand, binary exponent),
/// used for the default/`'a'`/`'A'` presentation types so no base conversion rounding occurs.
pub fn format_fbig_hex(f: &FBig, spec_str: &str) -> PyResult<String> {
    let s = parse(spec_str)?;
    let upper = s.ty == 'A';
    let body = match s.prec {
        Some(p) => {
            if upper {
                format!("{:.*X}", p, f)
            } else {
                format!("{:.*x}", p, f)
            }
        }
        None => {
            if upper {
                format!("{:X}", f)
            } else {
                format!("{:x}", f)
            }
        }
    };
    // split the sign and the `0x` prefix off, so that the sign option applies and the padding
    // of the '=' alignment (and so of the zero-padding) goes between the prefix and the digits
    let (negative, body) = strip_sign(&body);
    let (prefix, digits) = if body.starts_with("0x") || body.starts_with("0X") {
        body.split_at(2)
    } else {
        ("", body.as_str())
    };
    Ok(apply_layout(negative, prefix, digits, false, &s))
}

/// Split a leading '-' from a dashu-rendered string; return (is_negative, magnitude_str).
fn strip_sign(s: &str) -> (bool, String) {
    if let Some(rest) = s.strip_prefix('-') {
        (true, rest.to_string())
    } else if let Some(rest) = s.strip_prefix('+') {
        (false, rest.to_string())
    } else {
        (false, s.to_string())
    }
}

/// Group the digits of an integer part by 3 with the given separator, and pad it with zeros
/// on the left up to `min_width` characters. As in CPython, the padding zeros are grouped as
/// well, and the result never starts with a separator (it is one character wider instead).
fn group_digits(digits: &str, group: Option<char>, min_width: usize) -> String {
    let sep = match group {
        Some(sep) => sep,
        None => return format!("{digits:0>min_width$}"),
    };

    let mut reversed: Vec<char> = Vec::new();
    let mut group_len = 0;
    let mut source = digits.chars().rev();
    loop {
        let digit = match source.next() {
            Some(c) => c,
            None if reversed.len() < min_width => '0',
            None => break,
        };
        if group_len == 3 {
            reversed.push(sep);
            group_len = 0;
        }
        reversed.push(digit);
        group_len += 1;
    }
    reversed.into_iter().rev().collect()
}

/// Assemble the final string: sign (per the sign option), `prefix` (such as `0x`), and the
/// body, with the grouping (for a decimal `numeric` body, whose leading digits are the integer
/// part) and the width / align / fill / zero-pad of the spec.
fn apply_layout(negative: bool, prefix: &str, body: &str, numeric: bool, s: &Spec) -> String {
    let sign = if negative {
        "-"
    } else {
        match s.sign {
            '+' => "+",
            ' ' => " ",
            _ => "",
        }
    };
    let (fill, align) = if s.zero {
        ('0', s.align.unwrap_or('='))
    } else {
        (s.fill, s.align.unwrap_or('>'))
    };
    let width = s.width.unwrap_or(0);

    // the integer digits take the grouping; zero-padding between the sign and the digits
    // extends them (so that it is grouped too)
    let int_len = if numeric {
        body.chars().take_while(|c| c.is_ascii_digit()).count()
    } else {
        0
    };
    let (int_digits, rest) = body.split_at(int_len);
    let min_int_width = if int_len > 0 && fill == '0' && align == '=' {
        width.saturating_sub(sign.len() + prefix.len() + rest.chars().count())
    } else {
        0
    };
    let int_part = if int_len > 0 {
        group_digits(int_digits, s.group, min_int_width)
    } else {
        String::new()
    };

    let len = sign.len() + prefix.len() + int_part.chars().count() + rest.chars().count();
    let pad = width.saturating_sub(len);
    let fill_str = |n: usize| std::iter::repeat_n(fill, n).collect::<String>();
    match align {
        '<' => format!("{sign}{prefix}{int_part}{rest}{}", fill_str(pad)),
        '^' => {
            let half = pad / 2;
            format!("{}{sign}{prefix}{int_part}{rest}{}", fill_str(half), fill_str(pad - half))
        }
        // padding after the sign and the prefix
        '=' => format!("{sign}{prefix}{}{int_part}{rest}", fill_str(pad)),
        _ => format!("{}{sign}{prefix}{int_part}{rest}", fill_str(pad)),
    }
}

/// Normalize dashu's bare scientific exponent (`1.5e0`) to CPython's form (`1.5e+00`).
fn normalize_sci(s: String, upper: bool) -> String {
    let idx = s.find(['e', 'E']);
    match idx {
        Some(i) => {
            let (head, tail) = s.split_at(i);
            let exp_part = &tail[1..];
            let exp: isize = exp_part.parse().unwrap_or(0);
            let marker = if upper { 'E' } else { 'e' };
            format!("{head}{marker}{exp:+03}")
        }
        None => s,
    }
}
