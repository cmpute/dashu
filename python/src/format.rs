//! Python format mini-language (`__format__`) support.
//!
//! Integers (`UBig`/`IBig`) delegate to Python's own `int.__format__` (Python ints are
//! arbitrary precision, so there is no loss). Floats (`FBig`/`DBig`) are rendered in decimal
//! (an `FBig` is first converted to base 10) using dashu's precision-aware formatting, then
//! the Python spec's layout (sign / width / align / fill / zero-pad / grouping) and the
//! scientific exponent are normalized to CPython conventions.

use dashu_float::DBig;
use pyo3::PyResult;
use pyo3::exceptions::PyValueError;

/// A parsed Python format spec: `[[fill]align][sign][#][0][width][grouping][.precision][type]`.
#[allow(dead_code)]
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
    let mut align = None;
    if n >= 2 && "<>=^".contains(chars[1]) {
        fill = chars[0];
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

    // [0]
    let mut zero = false;
    if i < n && chars[i] == '0' {
        zero = true;
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
    let s = parse(spec_str)?;
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

    // Produce the unsigned (no sign policy) numeric body in decimal.
    let (negative, body) = render_body(d, &s)?;
    let signed = apply_sign(body, negative, s.sign);
    let grouped = apply_grouping(signed, s.group);
    Ok(apply_layout(grouped, &s))
}

/// Render the magnitude as a decimal string (without sign), per the spec's type.
fn render_body(d: &DBig, s: &Spec) -> PyResult<(bool, String)> {
    // infinite values: let dashu render, then strip the sign
    let (neg, raw) = match s.ty {
        'e' | 'E' => {
            let p = s.prec.unwrap_or(6);
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
            let (neg, mut body) = strip_sign(&format!("{:.*}", p, &scaled));
            body.push('%');
            (neg, body)
        }
        'g' | 'G' | 'n' | '\0' => {
            // general / default: full plain decimal (dashu Display), with optional precision
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

/// Reattach the sign according to the Python sign option ('+', '-', ' ').
fn apply_sign(body: String, negative: bool, sign: char) -> String {
    if negative {
        format!("-{body}")
    } else {
        match sign {
            '+' => format!("+{body}"),
            ' ' => format!(" {body}"),
            _ => body,
        }
    }
}

/// Insert a grouping separator every 3 digits in the integer part.
fn apply_grouping(mut body: String, group: Option<char>) -> String {
    let sep = match group {
        Some(c) => c,
        None => return body,
    };
    // locate the integer-part digits: from start (or after a leading sign) up to '.'/'e'/'E'/'%'
    let bytes: Vec<char> = body.chars().collect();
    let start = if bytes
        .first()
        .map(|c| *c == '-' || *c == '+')
        .unwrap_or(false)
    {
        1
    } else {
        0
    };
    let end = bytes
        .iter()
        .enumerate()
        .skip(start)
        .find(|(_, c)| **c == '.' || **c == 'e' || **c == 'E' || **c == '%')
        .map(|(i, _)| i)
        .unwrap_or(bytes.len());
    if end <= start {
        return body;
    }
    let int_digits: Vec<char> = bytes[start..end].to_vec();
    let m = int_digits.len();
    let mut grouped = String::new();
    for (k, c) in int_digits.iter().enumerate() {
        if k > 0 && (m - k) % 3 == 0 {
            grouped.push(sep);
        }
        grouped.push(*c);
    }
    // reassemble
    let prefix: String = bytes[..start].iter().collect();
    let suffix: String = bytes[end..].iter().collect();
    body = format!("{prefix}{grouped}{suffix}");
    body
}

/// Apply width / align / fill / zero-pad.
fn apply_layout(mut body: String, s: &Spec) -> String {
    let width = match s.width {
        Some(w) => w,
        None => return body,
    };
    let pad = width.saturating_sub(body.chars().count());
    if pad == 0 {
        return body;
    }
    let (fill, align) = if s.zero {
        ('0', s.align.unwrap_or('='))
    } else {
        (s.fill, s.align.unwrap_or('>'))
    };
    let pad_str: String = std::iter::repeat_n(fill, pad).collect();
    match align {
        '<' => body.push_str(&pad_str),
        '>' => body = format!("{pad_str}{body}"),
        '^' => {
            let half = pad / 2;
            body = format!(
                "{}{}{}",
                std::iter::repeat_n(fill, half).collect::<String>(),
                body,
                std::iter::repeat_n(fill, pad - half).collect::<String>()
            );
        }
        '=' => {
            // padding after the sign
            let (sign, rest) = if body.starts_with('-') || body.starts_with('+') {
                body.split_at(1)
            } else {
                ("", body.as_str())
            };
            body = format!("{sign}{pad_str}{rest}");
        }
        _ => {}
    }
    body
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
