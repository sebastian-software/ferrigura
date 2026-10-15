//! String helpers that behave like the JavaScript regular expressions and string methods the
//! TypeScript reference uses.
//!
//! JavaScript's `\s` is a fixed list of code points, which is not Rust's `char::is_whitespace`
//! (that one includes U+0085 and omits U+FEFF), so the list is spelled out here.

/// Whether `c` matches JavaScript's `\s`.
pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// Whether `c` is a line terminator for the `m` flag, i.e. what `^` and `$` match around.
pub fn is_line_term(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// Whether `c` matches `[\n\r]`.
pub fn is_cr_lf(c: char) -> bool {
    c == '\n' || c == '\r'
}

/// Whether `c` matches `\w`, which in JavaScript is ASCII only.
pub fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `String.prototype.trim` with JavaScript's whitespace.
pub fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

/// `replace(/[\t ]{2,}/g, ' ')`: each run of at least two tabs or spaces becomes one space.
pub fn collapse_blanks(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut run = String::new();
    for c in s.chars() {
        if c == '\t' || c == ' ' {
            run.push(c);
            continue;
        }
        flush_blank_run(&mut out, &mut run);
        out.push(c);
    }
    flush_blank_run(&mut out, &mut run);
    out
}

fn flush_blank_run(out: &mut String, run: &mut String) {
    match run.chars().count() {
        0 => {}
        1 => out.push_str(run),
        _ => out.push(' '),
    }
    run.clear();
}

/// `replace(/[\n\r]{2,}/g, '\n')`: each run of at least two line breaks becomes one newline.
pub fn collapse_line_breaks(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut run = String::new();
    for c in s.chars() {
        if is_cr_lf(c) {
            run.push(c);
            continue;
        }
        flush_line_break_run(&mut out, &mut run);
        out.push(c);
    }
    flush_line_break_run(&mut out, &mut run);
    out
}

fn flush_line_break_run(out: &mut String, run: &mut String) {
    // A single line break is kept as written; only a run of two or more is replaced.
    match run.chars().count() {
        0 => {}
        1 => out.push_str(run),
        _ => out.push('\n'),
    }
    run.clear();
}

/// `replace(/^\s*/gm, '')`: removes the whitespace at the start of every line, including the
/// line breaks that a `\s*` run swallows.
pub fn strip_line_starts(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        let at_line_start = i == 0 || is_line_term(chars[i - 1]);
        if at_line_start {
            let mut end = i;
            while end < chars.len() && is_js_space(chars[end]) {
                end += 1;
            }
            if end > i {
                i = end;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// `replace(/\s+$/gm, '')`: removes the whitespace at the end of every line.
///
/// The regular expression is greedy, so a run of whitespace that spans several line breaks is
/// cut at the last line break inside it: that line break and everything after it stay.
pub fn strip_line_ends(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < n {
        if !is_js_space(chars[i]) {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let mut end = i;
        while end < n && is_js_space(chars[end]) {
            end += 1;
        }
        let cut = if end == n {
            Some(end)
        } else {
            (i + 1..end).rev().find(|&k| is_line_term(chars[k]))
        };
        match cut {
            Some(p) => i = p,
            None => {
                out.extend(&chars[i..end]);
                i = end;
            }
        }
    }
    out
}

/// `replace(/^\s+/g, '')`: removes leading whitespace of the whole string.
pub fn strip_leading_space(s: &str) -> &str {
    s.trim_start_matches(is_js_space)
}

/// `replace(/\n\s+/g, '\n')`: each line break followed by whitespace becomes a single line break.
pub fn newline_then_space_to_newline(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        out.push(chars[i]);
        if chars[i] == '\n' {
            let mut end = i + 1;
            while end < chars.len() && is_js_space(chars[end]) {
                end += 1;
            }
            i = end;
            continue;
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_whitespace_around_line_breaks_like_the_js_regexes() {
        assert_eq!(
            strip_line_starts("\n        a\n        b\n      "),
            "a\nb\n"
        );
        assert_eq!(strip_line_ends("a \n  \nb"), "a\nb");
        assert_eq!(strip_line_ends("lines\n"), "lines");
        assert_eq!(newline_then_space_to_newline("a\n  \n  b"), "a\nb");
    }

    #[test]
    fn collapses_runs() {
        assert_eq!(collapse_blanks("a  \t b c"), "a b c");
        assert_eq!(collapse_line_breaks("a\n\r\nb\nc"), "a\nb\nc");
    }
}
