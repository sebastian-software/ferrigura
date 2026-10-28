//! A port of the part of marked 16.3.0 that `markdownToLines` reads: `marked.lexer` with its
//! defaults (GFM, no `breaks`, not pedantic), as Mermaid 12.2.0 calls it in
//! `packages/mermaid/src/rendering-util/handle-markdown-text.ts`.
//!
//! The lexer is ported from marked's `Lexer.ts` and `Tokenizer.ts`, and its rules from `rules.ts`:
//! the patterns are built with the same `edit` substitutions, then translated from JavaScript's
//! regular expressions to Oniguruma's ([`js`]) and compiled with ferroni. Only what the lines and
//! words depend on is kept: the type and raw text of each top-level token, and of a paragraph its
//! inline tokens, with text, strong, emphasis and HTML apart and every other inline token as
//! [`Inline::Other`]. The rest of the lexer runs as marked runs it, because it decides those
//! tokens: link definitions, the extent of lists and block quotes, the paragraph continuations, and
//! the lexer state that the inline tokens carry from one to the next.
//!
//! marked works on UTF-16 code units and this port on UTF-8 bytes. Where marked's result depends
//! on the units, the port counts them: the masks of the inline lexer have marked's lengths, and
//! the emphasis tokenizer adds up its positions in units, so that an escaped symbol outside the
//! Basic Multilingual Plane, which shortens marked's mask by one unit, shifts its emphasis as it
//! does there. The character before a delimiter run is a lone surrogate in marked when the text has
//! a character outside that plane there, which the port keeps as `PrevChar::Surrogate`.

use std::collections::HashSet;
use std::sync::LazyLock;

use ferroni::Regex;

/// A top-level token, as `markdownToLines` reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Block {
    /// A paragraph and its inline tokens.
    Paragraph(Vec<Inline>),
    /// A block of HTML, with its text.
    Html(String),
    /// Any other block, with its raw text.
    Other(String),
}

/// An inline token, as `markdownToLines` reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Inline {
    /// Text, merged with the text tokens next to it as marked merges them.
    Text(String),
    /// Strong emphasis and its tokens.
    Strong(Vec<Inline>),
    /// Emphasis and its tokens.
    Em(Vec<Inline>),
    /// Inline HTML, with its text.
    Html(String),
    /// An escape, a code span, a link, an image, a line break, a deletion or an autolink.
    Other,
}

/// `marked.lexer(src)`: the top-level tokens of a Markdown text.
pub(crate) fn lex(src: &str) -> Vec<Block> {
    let src = src.replace("\r\n", "\n").replace('\r', "\n");
    let mut lexer = Lexer::new();
    let mut tokens = Vec::new();
    lexer.block_tokens(&src, &mut tokens, false);
    for index in 0..lexer.queue.len() {
        let source = std::mem::take(&mut lexer.queue[index].src);
        let inline = lexer.inline_tokens(&source);
        lexer.queue[index].tokens = inline;
    }
    tokens
        .into_iter()
        .map(|token| match token.kind {
            Kind::Paragraph => Block::Paragraph(
                token
                    .job
                    .map(|job| std::mem::take(&mut lexer.queue[job].tokens))
                    .unwrap_or_default(),
            ),
            Kind::Html => Block::Html(token.text),
            _ => Block::Other(token.raw),
        })
        .collect()
}

// --- Rules ---------------------------------------------------------------------------------------

/// JavaScript's `\s`: the white space and line terminators of ECMAScript, which is not Oniguruma's.
const JS_SPACE: &str = r"\t\n\x{0B}\f\r \x{A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}";

/// JavaScript's `.`: any character but a line terminator.
const JS_DOT: &str = r"[^\n\r\x{2028}\x{2029}]";

/// JavaScript's `\w`, which is ASCII.
const JS_WORD: &str = "A-Za-z0-9_";

/// Translates a JavaScript pattern, with its flags, to Oniguruma's syntax with the same meaning:
/// `^` and `$` are the ends of the text unless the flags have `m`, `.`, `\s`, `\w`, `\d` and `\b`
/// are JavaScript's, and a literal `[`, `&` or `-` in a class is escaped where Oniguruma would read
/// it as a nested class, an intersection or a range.
fn js(source: &str, flags: &str) -> String {
    let multiline = flags.contains('m');
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len() * 2);
    if flags.contains('i') {
        out.push_str("(?i)");
    }
    let mut index = 0;
    let mut in_class = false;
    // In a class: whether the last item was one character, which a `-` may start a range from.
    let mut last_single = false;
    let mut in_range = false;
    while index < chars.len() {
        let character = chars[index];
        if in_class {
            match character {
                '\\' => {
                    index += 1;
                    let escaped = chars.get(index).copied().unwrap_or('\\');
                    let single = match escaped {
                        's' => {
                            out.push_str(JS_SPACE);
                            false
                        }
                        'S' => {
                            out.push_str(&format!("[^{JS_SPACE}]"));
                            false
                        }
                        'w' => {
                            out.push_str(JS_WORD);
                            false
                        }
                        'W' => {
                            out.push_str(&format!("[^{JS_WORD}]"));
                            false
                        }
                        'd' => {
                            out.push_str("0-9");
                            false
                        }
                        'D' => {
                            out.push_str("[^0-9]");
                            false
                        }
                        'p' | 'P' => {
                            index = copy_property(&chars, index, &mut out);
                            false
                        }
                        'x' => {
                            index = copy_hex(&chars, index, &mut out);
                            true
                        }
                        other => {
                            out.push('\\');
                            out.push(other);
                            true
                        }
                    };
                    end_class_item(single, &mut last_single, &mut in_range);
                }
                ']' => {
                    out.push(']');
                    in_class = false;
                }
                '-' => {
                    let next = chars.get(index + 1).copied();
                    if last_single && !in_range && next.is_some_and(|next| next != ']') {
                        out.push('-');
                        in_range = true;
                    } else {
                        out.push_str("\\-");
                        end_class_item(true, &mut last_single, &mut in_range);
                    }
                }
                '[' | '&' => {
                    out.push('\\');
                    out.push(character);
                    end_class_item(true, &mut last_single, &mut in_range);
                }
                other => {
                    out.push(other);
                    end_class_item(true, &mut last_single, &mut in_range);
                }
            }
        } else {
            match character {
                '\\' => {
                    index += 1;
                    let escaped = chars.get(index).copied().unwrap_or('\\');
                    match escaped {
                        's' => out.push_str(&format!("[{JS_SPACE}]")),
                        'S' => out.push_str(&format!("[^{JS_SPACE}]")),
                        'w' => out.push_str(&format!("[{JS_WORD}]")),
                        'W' => out.push_str(&format!("[^{JS_WORD}]")),
                        'd' => out.push_str("[0-9]"),
                        'D' => out.push_str("[^0-9]"),
                        'b' => out.push_str(&format!(
                            "(?:(?<=[{JS_WORD}])(?![{JS_WORD}])|(?<![{JS_WORD}])(?=[{JS_WORD}]))"
                        )),
                        'B' => out.push_str(&format!(
                            "(?:(?<=[{JS_WORD}])(?=[{JS_WORD}])|(?<![{JS_WORD}])(?![{JS_WORD}]))"
                        )),
                        'p' | 'P' => index = copy_property(&chars, index, &mut out),
                        'x' => index = copy_hex(&chars, index, &mut out),
                        other => {
                            out.push('\\');
                            out.push(other);
                        }
                    }
                }
                '[' => {
                    out.push('[');
                    in_class = true;
                    last_single = false;
                    in_range = false;
                    if chars.get(index + 1) == Some(&'^') {
                        out.push('^');
                        index += 1;
                    }
                }
                '^' => out.push_str(if multiline { "^" } else { r"\A" }),
                '$' => out.push_str(if multiline { "$" } else { r"\z" }),
                '.' => out.push_str(JS_DOT),
                other => out.push(other),
            }
        }
        index += 1;
    }
    out
}

/// After an item of a class: one character, or a set or the end of a range.
fn end_class_item(single: bool, last_single: &mut bool, in_range: &mut bool) {
    if *in_range {
        *in_range = false;
        *last_single = false;
    } else {
        *last_single = single;
    }
}

/// Copies `\p{…}` or `\P{…}`, whose letter is at `index`; returns the index of its `}`.
fn copy_property(chars: &[char], index: usize, out: &mut String) -> usize {
    out.push('\\');
    out.push(chars[index]);
    let mut end = index + 1;
    while end < chars.len() {
        out.push(chars[end]);
        if chars[end] == '}' {
            break;
        }
        end += 1;
    }
    end
}

/// Copies `\xHH`, whose `x` is at `index`; returns the index of its last digit.
fn copy_hex(chars: &[char], index: usize, out: &mut String) -> usize {
    out.push_str(r"\x");
    let mut end = index;
    for offset in 1..=2 {
        if let Some(digit) = chars.get(index + offset) {
            out.push(*digit);
            end = index + offset;
        }
    }
    end
}

/// `other.caret` in `rules.ts`, `/(^|[^\[])\^/g` replaced by `$1`: an inserted source loses its
/// carets, except those right after `[`.
fn strip_carets(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut index = 0;
    while index < chars.len() {
        if index == 0 && chars[0] == '^' {
            index += 1;
        } else if chars[index] != '[' && chars.get(index + 1) == Some(&'^') {
            out.push(chars[index]);
            index += 2;
        } else {
            out.push(chars[index]);
            index += 1;
        }
    }
    out
}

/// `edit` in `rules.ts`: a source and its substitutions. `replace` takes the first occurrence of
/// a name, as a string pattern does in JavaScript, and `replace_all` every occurrence, as a global
/// pattern does; the inserted source loses its carets first.
struct Edit(String);

impl Edit {
    fn new(source: &str) -> Self {
        Self(source.to_owned())
    }

    fn replace(mut self, name: &str, value: &str) -> Self {
        let value = strip_carets(value);
        if let Some(position) = self.0.find(name) {
            self.0
                .replace_range(position..position + name.len(), &value);
        }
        self
    }

    fn replace_all(mut self, name: &str, value: &str) -> Self {
        let value = strip_carets(value);
        self.0 = self.0.replace(name, &value);
        self
    }

    fn get(self) -> String {
        self.0
    }
}

fn compile(source: &str, flags: &str) -> Regex {
    let pattern = js(source, flags);
    Regex::new(&pattern)
        .unwrap_or_else(|error| panic!("marked rule {source:?} as {pattern:?}: {error}"))
}

/// The sources of `rules.ts` that later rules insert, verbatim.
mod source {
    pub(super) const NEWLINE: &str = r"^(?:[ \t]*(?:\n|$))+";
    pub(super) const BLOCK_CODE: &str = r"^((?: {4}| {0,3}\t)[^\n]+(?:\n(?:[ \t]*(?:\n|$))*)?)+";
    pub(super) const FENCES: &str = r"^ {0,3}(`{3,}(?=[^`\n]*(?:\n|$))|~{3,})([^\n]*)(?:\n|$)(?:|([\s\S]*?)(?:\n|$))(?: {0,3}\1[~`]* *(?=\n|$)|$)";
    pub(super) const HR: &str =
        r"^ {0,3}((?:-[\t ]*){3,}|(?:_[ \t]*){3,}|(?:\*[ \t]*){3,})(?:\n+|$)";
    pub(super) const HEADING: &str = r"^ {0,3}(#{1,6})(?=\s|$)(.*)(?:\n+|$)";
    pub(super) const BULLET: &str = r"(?:[*+-]|\d{1,9}[.)])";
    pub(super) const LHEADING_CORE: &str = r"^(?!bull |blockCode|fences|blockquote|heading|html|table)((?:.|\n(?!\s*?\n|bull |blockCode|fences|blockquote|heading|html|table))+?)\n {0,3}(=+|-+) *(?:\n+|$)";
    pub(super) const PARAGRAPH: &str =
        r"^([^\n]+(?:\n(?!hr|heading|lheading|blockquote|fences|list|html|table| +\n)[^\n]+)*)";
    pub(super) const BLOCK_TEXT: &str = r"^[^\n]+";
    pub(super) const BLOCK_LABEL: &str = r"(?!\s*\])(?:\\[\s\S]|[^\[\]\\])+";
    pub(super) const DEF: &str = r"^ {0,3}\[(label)\]: *(?:\n[ \t]*)?([^<\s][^\s]*|<.*?>)(?:(?: +(?:\n[ \t]*)?| *\n[ \t]*)(title))? *(?:\n+|$)";
    pub(super) const DEF_TITLE: &str =
        r##"(?:"(?:\\"?|[^"\\])*"|'[^'\n]*(?:\n[^'\n]+)*\n?'|\([^()]*\))"##;
    pub(super) const LIST: &str = r"^( {0,3}bull)([ \t][^\n]+?)?(?:\n|$)";
    pub(super) const TAG_NAMES: &str = "address|article|aside|base|basefont|blockquote|body|caption|center|col|colgroup|dd|details|dialog|dir|div|dl|dt|fieldset|figcaption|figure|footer|form|frame|frameset|h[1-6]|head|header|hr|html|iframe|legend|li|link|main|menu|menuitem|meta|nav|noframes|ol|optgroup|option|p|param|search|section|summary|table|tbody|td|tfoot|th|thead|title|tr|track|ul";
    pub(super) const COMMENT: &str = r"<!--(?:-?>|[\s\S]*?(?:-->|$))";
    pub(super) const HTML: &str = concat!(
        r"^ {0,3}(?:",
        r"<(script|pre|style|textarea)[\s>][\s\S]*?(?:</\1>[^\n]*\n+|$)",
        r"|comment[^\n]*(\n+|$)",
        r"|<\?[\s\S]*?(?:\?>\n*|$)",
        r"|<![A-Z][\s\S]*?(?:>\n*|$)",
        r"|<!\[CDATA\[[\s\S]*?(?:\]\]>\n*|$)",
        r"|</?(tag)(?: +|\n|/?>)[\s\S]*?(?:(?:\n[ \t]*)+\n|$)",
        r"|<(?!script|pre|style|textarea)([a-z][\w-]*)(?:attribute)*? */?>(?=[ \t]*(?:\n|$))[\s\S]*?(?:(?:\n[ \t]*)+\n|$)",
        r"|</(?!script|pre|style|textarea)[a-z][\w-]*\s*>(?=[ \t]*(?:\n|$))[\s\S]*?(?:(?:\n[ \t]*)+\n|$)",
        r")",
    );
    pub(super) const HTML_ATTRIBUTE: &str =
        r##" +[a-zA-Z:_][\w.:-]*(?: *= *"[^"\n]*"| *= *'[^'\n]*'| *= *[^\s"'=<>`]+)?"##;
    pub(super) const HEADING_INTERRUPT: &str = r" {0,3}#{1,6}(?:\s|$)";
    pub(super) const BLOCKQUOTE_INTERRUPT: &str = r" {0,3}>";
    pub(super) const FENCES_INTERRUPT: &str = r" {0,3}(?:`{3,}(?=[^`\n]*\n)|~{3,})[^\n]*\n";
    pub(super) const LIST_INTERRUPT: &str = r" {0,3}(?:[*+-]|1[.)]) ";
    pub(super) const HTML_INTERRUPT: &str =
        r"</?(?:tag)(?: +|\n|/?>)|<(?:script|pre|style|textarea|!--)";
    pub(super) const BLOCKQUOTE: &str = r"^( {0,3}> ?(paragraph|[^\n]*)(?:\n|$))+";
    pub(super) const GFM_TABLE: &str = r"^ *([^\n ].*)\n {0,3}((?:\| *)?:?-+:? *(?:\| *:?-+:? *)*(?:\| *)?)(?:\n((?:(?! *\n|hr|heading|blockquote|code|fences|list|html).*(?:\n|$))*)\n*|$)";

    pub(super) const ESCAPE: &str = r##"^\\([!"#$%&'()*+,\-./:;<=>?@\[\]\\^_`{|}~])"##;
    pub(super) const INLINE_CODE: &str = r"^(`+)([^`]|[^`][\s\S]*?[^`])\1(?!`)";
    pub(super) const BR: &str = r"^( {2,}|\\)\n(?!\s*$)";
    pub(super) const PUNCTUATION: &str = r"[\p{P}\p{S}]";
    pub(super) const PUNCTUATION_OR_SPACE: &str = r"[\s\p{P}\p{S}]";
    pub(super) const NOT_PUNCTUATION_OR_SPACE: &str = r"[^\s\p{P}\p{S}]";
    pub(super) const PUNCTUATION_GFM: &str = r"(?!~)[\p{P}\p{S}]";
    pub(super) const PUNCTUATION_OR_SPACE_GFM: &str = r"(?!~)[\s\p{P}\p{S}]";
    pub(super) const NOT_PUNCTUATION_OR_SPACE_GFM: &str = r"(?:[^\s\p{P}\p{S}]|~)";
    pub(super) const BLOCK_SKIP: &str = r"\[[^\[\]]*?\]\((?:\\[\s\S]|[^\\\(\)]|\((?:\\[\s\S]|[^\\\(\)])*\))*\)|`[^`]*?`|<(?! )[^<>]*?>";
    pub(super) const EM_STRONG_L_DELIM: &str =
        r"^(?:\*+(?:((?!\*)punct)|[^\s*]))|^_+(?:((?!_)punct)|([^\s_]))";
    pub(super) const EM_STRONG_R_DELIM_AST: &str = concat!(
        r"^[^_*]*?__[^_*]*?\*[^_*]*?(?=__)",
        r"|[^*]+(?=[^*])",
        r"|(?!\*)punct(\*+)(?=[\s]|$)",
        r"|notPunctSpace(\*+)(?!\*)(?=punctSpace|$)",
        r"|(?!\*)punctSpace(\*+)(?=notPunctSpace)",
        r"|[\s](\*+)(?!\*)(?=punct)",
        r"|(?!\*)punct(\*+)(?!\*)(?=punct)",
        r"|notPunctSpace(\*+)(?=notPunctSpace)",
    );
    pub(super) const EM_STRONG_R_DELIM_UND: &str = concat!(
        r"^[^_*]*?\*\*[^_*]*?_[^_*]*?(?=\*\*)",
        r"|[^_]+(?=[^_])",
        r"|(?!_)punct(_+)(?=[\s]|$)",
        r"|notPunctSpace(_+)(?!_)(?=punctSpace|$)",
        r"|(?!_)punctSpace(_+)(?=notPunctSpace)",
        r"|[\s](_+)(?!_)(?=punct)",
        r"|(?!_)punct(_+)(?!_)(?=punct)",
    );
    pub(super) const AUTOLINK: &str = r"^<(scheme:[^\s\x00-\x1f<>]*|email)>";
    pub(super) const AUTOLINK_SCHEME: &str = r"[a-zA-Z][a-zA-Z0-9+.-]{1,31}";
    pub(super) const AUTOLINK_EMAIL: &str = r"[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+(@)[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)+(?![-_])";
    pub(super) const TAG: &str = concat!(
        r"^comment",
        r"|^</[a-zA-Z][\w:-]*\s*>",
        r"|^<[a-zA-Z][\w-]*(?:attribute)*?\s*/?>",
        r"|^<\?[\s\S]*?\?>",
        r"|^<![a-zA-Z]+\s[\s\S]*?>",
        r"|^<!\[CDATA\[[\s\S]*?\]\]>",
    );
    pub(super) const TAG_ATTRIBUTE: &str =
        r##"\s+[a-zA-Z:_][\w.:-]*(?:\s*=\s*"[^"]*"|\s*=\s*'[^']*'|\s*=\s*[^\s"'=<>`]+)?"##;
    pub(super) const INLINE_LABEL: &str =
        r"(?:\[(?:\\[\s\S]|[^\[\]\\])*\]|\\[\s\S]|`[^`]*`|[^\[\]\\`])*?";
    pub(super) const LINK: &str =
        r"^!?\[(label)\]\(\s*(href)(?:(?:[ \t]*(?:\n[ \t]*)?)(title))?\s*\)";
    pub(super) const LINK_HREF: &str = r"<(?:\\.|[^\n<>\\])+>|[^ \t\n\x00-\x1f]*";
    pub(super) const LINK_TITLE: &str =
        r##""(?:\\"?|[^"\\])*"|'(?:\\'?|[^'\\])*'|\((?:\\\)?|[^)\\])*\)"##;
    pub(super) const REFLINK: &str = r"^!?\[(label)\]\[(ref)\]";
    pub(super) const NOLINK: &str = r"^!?\[(ref)\](?:\[\])?";
    pub(super) const URL: &str =
        r"^((?:ftp|https?):\/\/|www\.)(?:[a-zA-Z0-9\-]+\.?)+[^\s<]*|^email";
    pub(super) const URL_EMAIL: &str =
        r"[A-Za-z0-9._+-]+(@)[a-zA-Z0-9-_]+(?:\.[a-zA-Z0-9-_]*[a-zA-Z0-9])+(?![-_])";
    pub(super) const BACKPEDAL: &str =
        r##"(?:[^?!.,:;*_'"~()&]+|\([^)]*\)|&(?![a-zA-Z0-9]+;$)|[?!.,:;*_'"~)]+(?!$))+"##;
    pub(super) const DEL: &str =
        r"^(~~?)(?=[^\s~])((?:\\[\s\S]|[^\\])*?(?:\\[\s\S]|[^\s~\\]))\1(?=[^~]|$)";
    pub(super) const TEXT: &str = r"^([`~]+|[^`~])(?:(?= {2,}\n)|(?=[a-zA-Z0-9.!#$%&'*+\/=?_`{\|}~-]+@)|[\s\S]*?(?:(?=[\\<!\[`*~_]|\b_|https?:\/\/|ftp:\/\/|www\.|$)|[^ ](?= {2,}\n)|[^a-zA-Z0-9.!#$%&'*+\/=?_`{\|}~-](?=[a-zA-Z0-9.!#$%&'*+\/=?_`{\|}~-]+@)))";
}

/// The compiled rules of the GFM grammar, and the patterns of `other` that the tokenizer uses.
struct Rules {
    newline: Regex,
    code: Regex,
    fences: Regex,
    hr: Regex,
    heading: Regex,
    blockquote: Regex,
    list: Regex,
    html: Regex,
    def: Regex,
    table: Regex,
    lheading: Regex,
    paragraph: Regex,
    text: Regex,

    escape: Regex,
    tag: Regex,
    link: Regex,
    reflink: Regex,
    nolink: Regex,
    reflink_search: Regex,
    em_strong_l_delim: Regex,
    em_strong_r_delim_ast: Regex,
    em_strong_r_delim_und: Regex,
    punctuation: Regex,
    code_span: Regex,
    br: Regex,
    del: Regex,
    autolink: Regex,
    url: Regex,
    backpedal: Regex,
    inline_text: Regex,
    any_punctuation: Regex,
    block_skip: Regex,

    blockquote_start: Regex,
    blockquote_setext: Regex,
    blockquote_marker: Regex,
    code_remove_indent: Regex,
    list_is_task: Regex,
    list_replace_task: Regex,
    table_align_chars: Regex,
    table_row_blank_line: Regex,
    start_a_tag: Regex,
    end_a_tag: Regex,
    unicode_alpha_numeric: Regex,
}

static RULES: LazyLock<Rules> = LazyLock::new(|| {
    use source as s;
    let hr = s::HR;
    let paragraph_normal = Edit::new(s::PARAGRAPH)
        .replace("hr", hr)
        .replace("heading", s::HEADING_INTERRUPT)
        .replace("|lheading", "")
        .replace("|table", "")
        .replace("blockquote", s::BLOCKQUOTE_INTERRUPT)
        .replace("fences", s::FENCES_INTERRUPT)
        .replace("list", s::LIST_INTERRUPT)
        .replace("html", s::HTML_INTERRUPT)
        .replace("tag", s::TAG_NAMES)
        .get();
    let gfm_table = Edit::new(s::GFM_TABLE)
        .replace("hr", hr)
        .replace("heading", s::HEADING_INTERRUPT)
        .replace("blockquote", s::BLOCKQUOTE_INTERRUPT)
        .replace("code", r"(?: {4}| {0,3}\t)[^\n]")
        .replace("fences", s::FENCES_INTERRUPT)
        .replace("list", s::LIST_INTERRUPT)
        .replace("html", s::HTML_INTERRUPT)
        .replace("tag", s::TAG_NAMES)
        .get();
    let paragraph_gfm = Edit::new(s::PARAGRAPH)
        .replace("hr", hr)
        .replace("heading", s::HEADING_INTERRUPT)
        .replace("|lheading", "")
        .replace("table", &gfm_table)
        .replace("blockquote", s::BLOCKQUOTE_INTERRUPT)
        .replace("fences", s::FENCES_INTERRUPT)
        .replace("list", s::LIST_INTERRUPT)
        .replace("html", s::HTML_INTERRUPT)
        .replace("tag", s::TAG_NAMES)
        .get();
    let lheading = Edit::new(s::LHEADING_CORE)
        .replace_all("bull", s::BULLET)
        .replace_all("blockCode", r"(?: {4}| {0,3}\t)")
        .replace_all("fences", r" {0,3}(?:`{3,}|~{3,})")
        .replace_all("blockquote", r" {0,3}>")
        .replace_all("heading", r" {0,3}#{1,6}")
        .replace_all("html", r" {0,3}<[^\n>]+>\n")
        .replace_all("table", r" {0,3}\|?(?:[:\- ]*\|)+[\:\- ]*\n")
        .get();
    let def = Edit::new(s::DEF)
        .replace("label", s::BLOCK_LABEL)
        .replace("title", s::DEF_TITLE)
        .get();
    let list = Edit::new(s::LIST).replace_all("bull", s::BULLET).get();
    let html = Edit::new(s::HTML)
        .replace("comment", s::COMMENT)
        .replace("tag", s::TAG_NAMES)
        .replace("attribute", s::HTML_ATTRIBUTE)
        .get();
    let blockquote = Edit::new(s::BLOCKQUOTE)
        .replace("paragraph", &paragraph_normal)
        .get();

    let em_strong_l_delim = Edit::new(s::EM_STRONG_L_DELIM)
        .replace_all("punct", s::PUNCTUATION_GFM)
        .get();
    let em_strong_r_delim_ast = Edit::new(s::EM_STRONG_R_DELIM_AST)
        .replace_all("notPunctSpace", s::NOT_PUNCTUATION_OR_SPACE_GFM)
        .replace_all("punctSpace", s::PUNCTUATION_OR_SPACE_GFM)
        .replace_all("punct", s::PUNCTUATION_GFM)
        .get();
    let em_strong_r_delim_und = Edit::new(s::EM_STRONG_R_DELIM_UND)
        .replace_all("notPunctSpace", s::NOT_PUNCTUATION_OR_SPACE)
        .replace_all("punctSpace", s::PUNCTUATION_OR_SPACE)
        .replace_all("punct", s::PUNCTUATION)
        .get();
    let punctuation = Edit::new(r"^((?![*_])punctSpace)")
        .replace_all("punctSpace", s::PUNCTUATION_OR_SPACE)
        .get();
    let any_punctuation = Edit::new(r"\\(punct)")
        .replace_all("punct", s::PUNCTUATION)
        .get();
    let autolink = Edit::new(s::AUTOLINK)
        .replace("scheme", s::AUTOLINK_SCHEME)
        .replace("email", s::AUTOLINK_EMAIL)
        .get();
    let inline_comment = Edit::new(s::COMMENT).replace("(?:-->|$)", "-->").get();
    let tag = Edit::new(s::TAG)
        .replace("comment", &inline_comment)
        .replace("attribute", s::TAG_ATTRIBUTE)
        .get();
    let link = Edit::new(s::LINK)
        .replace("label", s::INLINE_LABEL)
        .replace("href", s::LINK_HREF)
        .replace("title", s::LINK_TITLE)
        .get();
    let reflink = Edit::new(s::REFLINK)
        .replace("label", s::INLINE_LABEL)
        .replace("ref", s::BLOCK_LABEL)
        .get();
    let nolink = Edit::new(s::NOLINK).replace("ref", s::BLOCK_LABEL).get();
    let reflink_search = Edit::new(r"reflink|nolink(?!\()")
        .replace("reflink", &reflink)
        .replace("nolink", &nolink)
        .get();
    let url = Edit::new(s::URL).replace("email", s::URL_EMAIL).get();

    Rules {
        newline: compile(s::NEWLINE, ""),
        code: compile(s::BLOCK_CODE, ""),
        fences: compile(s::FENCES, ""),
        hr: compile(hr, ""),
        heading: compile(s::HEADING, ""),
        blockquote: compile(&blockquote, ""),
        list: compile(&list, ""),
        html: compile(&html, "i"),
        def: compile(&def, ""),
        table: compile(&gfm_table, ""),
        lheading: compile(&lheading, ""),
        paragraph: compile(&paragraph_gfm, ""),
        text: compile(s::BLOCK_TEXT, ""),

        escape: compile(s::ESCAPE, ""),
        tag: compile(&tag, ""),
        link: compile(&link, ""),
        reflink: compile(&reflink, ""),
        nolink: compile(&nolink, ""),
        reflink_search: compile(&reflink_search, "g"),
        em_strong_l_delim: compile(&em_strong_l_delim, "u"),
        em_strong_r_delim_ast: compile(&em_strong_r_delim_ast, "gu"),
        em_strong_r_delim_und: compile(&em_strong_r_delim_und, "gu"),
        punctuation: compile(&punctuation, "u"),
        code_span: compile(s::INLINE_CODE, ""),
        br: compile(s::BR, ""),
        del: compile(s::DEL, ""),
        autolink: compile(&autolink, ""),
        url: compile(&url, "i"),
        backpedal: compile(s::BACKPEDAL, ""),
        inline_text: compile(s::TEXT, ""),
        any_punctuation: compile(&any_punctuation, "gu"),
        block_skip: compile(s::BLOCK_SKIP, "g"),

        blockquote_start: compile(r"^ {0,3}>", ""),
        blockquote_setext: compile(r"\n {0,3}((?:=+|-+) *)(?=\n|$)", "g"),
        blockquote_marker: compile(r"^ {0,3}>[ \t]?", "gm"),
        code_remove_indent: compile(r"^(?: {1,4}| {0,3}\t)", "gm"),
        list_is_task: compile(r"^\[[ xX]\] ", ""),
        list_replace_task: compile(r"^\[[ xX]\] +", ""),
        table_align_chars: compile(r"^\||\| *$", "g"),
        table_row_blank_line: compile(r"\n[ \t]*$", ""),
        start_a_tag: compile(r"^<a ", "i"),
        end_a_tag: compile(r"^<\/a>", "i"),
        unicode_alpha_numeric: compile(r"[\p{L}\p{N}]", "u"),
    }
});

/// The patterns of `other` that depend on a list item's indentation, `min(3, indent - 1)` spaces.
struct ItemRules {
    next_bullet: Regex,
    hr: Regex,
    fences_begin: Regex,
    heading_begin: Regex,
    html_begin: Regex,
}

impl ItemRules {
    fn new(indent: usize) -> Self {
        let spaces = 3.min(indent.saturating_sub(1));
        Self {
            next_bullet: compile(
                &format!(r"^ {{0,{spaces}}}(?:[*+-]|\d{{1,9}}[.)])((?:[ \t][^\n]*)?(?:\n|$))"),
                "",
            ),
            hr: compile(
                &format!(r"^ {{0,{spaces}}}((?:- *){{3,}}|(?:_ *){{3,}}|(?:\* *){{3,}})(?:\n+|$)"),
                "",
            ),
            fences_begin: compile(&format!(r"^ {{0,{spaces}}}(?:```|~~~)"), ""),
            heading_begin: compile(&format!(r"^ {{0,{spaces}}}#"), ""),
            html_begin: compile(&format!(r"^ {{0,{spaces}}}<(?:[a-z].*>|!--)"), "i"),
        }
    }
}

// --- Helpers -------------------------------------------------------------------------------------

/// JavaScript's `\s`, which `trim` and the `\s` of the patterns use.
fn is_js_space(character: char) -> bool {
    matches!(
        character,
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

fn js_trim(text: &str) -> &str {
    text.trim_matches(is_js_space)
}

/// `rtrim` in `helpers.ts`: the text without the copies of `character` at its end.
fn rtrim(text: &str, character: char) -> &str {
    text.trim_end_matches(character)
}

/// The text from `start` on, with `start` moved back to a character boundary: marked slices at
/// UTF-16 positions, which are not always character boundaries in UTF-8.
fn tail(text: &str, start: usize) -> &str {
    let mut start = start.min(text.len());
    while !text.is_char_boundary(start) {
        start -= 1;
    }
    &text[start..]
}

/// The text up to `end`, with `end` moved back to a character boundary.
fn head(text: &str, end: usize) -> &str {
    let mut end = end.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// `str.search(/[^ ]/)`: the index of the first character that is not a space, if any.
fn first_non_space(text: &str) -> Option<usize> {
    text.find(|character| character != ' ')
}

/// The first line of a text, as `split('\n', 1)[0]` gives it.
fn first_line(text: &str) -> &str {
    text.split('\n').next().unwrap_or_default()
}

/// A pattern applied to the start of `text`: the captures of its match, which marked's anchored
/// rules only find there.
fn exec<'t>(regex: &Regex, text: &'t str) -> Option<ferroni::Captures<'t>> {
    regex.captures(text)
}

fn group<'t>(captures: &ferroni::Captures<'t>, index: usize) -> Option<&'t str> {
    captures.get(index).map(|found| found.as_str())
}

/// Every match of a global pattern replaced, as `String.replace` with a global pattern does.
fn replace_global(
    regex: &Regex,
    text: &str,
    replacement: impl Fn(&ferroni::Captures<'_>) -> String,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut position = 0;
    let mut last = 0;
    while position <= text.len() {
        let Some(captures) = regex.captures_at(text, position) else {
            break;
        };
        let Some(found) = captures.get(0) else { break };
        out.push_str(&text[last..found.start()]);
        out.push_str(&replacement(&captures));
        last = found.end();
        position = if found.end() > found.start() {
            found.end()
        } else {
            match text[found.end()..].chars().next() {
                Some(character) => {
                    out.push(character);
                    last = found.end() + character.len_utf8();
                    found.end() + character.len_utf8()
                }
                None => break,
            }
        };
    }
    out.push_str(&text[last.min(text.len())..]);
    out
}

/// The `$1` of a match, or an empty text.
fn dollar_one(captures: &ferroni::Captures<'_>) -> String {
    group(captures, 1).unwrap_or_default().to_owned()
}

/// `splitCells` in `helpers.ts`: the cells of a table row.
fn split_cells(row: &str, count: Option<usize>) -> Vec<String> {
    // An unescaped pipe gets a space before it, so that it is told from an escaped one.
    let mut spaced = String::with_capacity(row.len() + 8);
    let mut backslashes = 0;
    for character in row.chars() {
        if character == '|' {
            if backslashes % 2 == 1 {
                spaced.push('|');
            } else {
                spaced.push_str(" |");
            }
        } else {
            spaced.push(character);
        }
        backslashes = if character == '\\' {
            backslashes + 1
        } else {
            0
        };
    }
    let mut cells: Vec<String> = spaced.split(" |").map(str::to_owned).collect();
    if cells.first().is_some_and(|cell| js_trim(cell).is_empty()) {
        cells.remove(0);
    }
    if cells.last().is_some_and(|cell| js_trim(cell).is_empty()) {
        cells.pop();
    }
    if let Some(count) = count {
        cells.resize(count, String::new());
    }
    cells
        .iter()
        .map(|cell| js_trim(cell).replace("\\|", "|"))
        .collect()
}

/// `findClosingBracket` in `helpers.ts` for `()`: the index of the parenthesis that closes the
/// text's first level, `None` for -1 (no closing one), and `Err` for -2 (more opened than closed).
fn find_closing_bracket(text: &str) -> Result<Option<usize>, ()> {
    if !text.contains(')') {
        return Ok(None);
    }
    let mut level = 0i32;
    let mut characters = text.char_indices();
    while let Some((index, character)) = characters.next() {
        match character {
            '\\' => {
                characters.next();
            }
            '(' => level += 1,
            ')' => {
                level -= 1;
                if level < 0 {
                    return Ok(Some(index));
                }
            }
            _ => {}
        }
    }
    if level > 0 { Err(()) } else { Ok(None) }
}

// --- Lexer ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Space,
    Code,
    Heading,
    Hr,
    Blockquote,
    List,
    Html,
    Def,
    Table,
    Paragraph,
    Text,
}

/// A block token: its type, raw text and text, and the inline job of a paragraph or text.
#[derive(Debug, Clone)]
struct Token {
    kind: Kind,
    raw: String,
    text: String,
    job: Option<usize>,
}

impl Token {
    fn new(kind: Kind, raw: &str) -> Self {
        Self {
            kind,
            raw: raw.to_owned(),
            text: raw.to_owned(),
            job: None,
        }
    }
}

/// A text the inline lexer runs on once every block is read, as `lexer.inline` queues it.
#[derive(Debug, Default)]
struct Job {
    src: String,
    tokens: Vec<Inline>,
}

/// The character before a delimiter run, as marked's inline lexer tracks it.
#[derive(Debug, Clone, Copy)]
enum PrevChar {
    /// No text token came right before.
    None,
    /// The last character of the text token before.
    Char(char),
    /// The text token before ended with a character outside the Basic Multilingual Plane, whose
    /// last UTF-16 unit is a lone surrogate for marked: neither a letter nor punctuation.
    Surrogate,
}

struct Lexer {
    rules: &'static Rules,
    links: HashSet<String>,
    queue: Vec<Job>,
    top: bool,
    in_link: bool,
}

impl Lexer {
    fn new() -> Self {
        Self {
            rules: &RULES,
            links: HashSet::new(),
            queue: Vec::new(),
            top: true,
            in_link: false,
        }
    }

    /// `lexer.inline`: queues a text for the inline lexer and returns its job.
    fn inline(&mut self, src: &str) -> usize {
        self.queue.push(Job {
            src: src.to_owned(),
            tokens: Vec::new(),
        });
        self.queue.len() - 1
    }

    /// The text of the last queued job, as `this.inlineQueue.at(-1).src = …` sets it.
    fn set_last_job(&mut self, text: &str) {
        if let Some(job) = self.queue.last_mut() {
            job.src = text.to_owned();
        }
    }

    /// `blockTokens`: the block tokens of a text, appended to `tokens`.
    fn block_tokens(
        &mut self,
        src: &str,
        tokens: &mut Vec<Token>,
        mut last_paragraph_clipped: bool,
    ) {
        let rules = self.rules;
        let mut position = 0;
        while position < src.len() {
            let rest = &src[position..];

            // space
            if let Some(found) = exec(&rules.newline, rest).and_then(|c| c.get(0))
                && !found.as_str().is_empty()
            {
                let raw = found.as_str();
                position += raw.len();
                match tokens.last_mut() {
                    Some(last) if raw.len() == 1 => last.raw.push('\n'),
                    _ => tokens.push(Token::new(Kind::Space, raw)),
                }
                continue;
            }

            // code
            if let Some(found) = exec(&rules.code, rest).and_then(|c| c.get(0)) {
                let raw = found.as_str();
                position += raw.len();
                let text = replace_global(&rules.code_remove_indent, raw, |_| String::new());
                let text = rtrim(&text, '\n').to_owned();
                match tokens.last_mut() {
                    Some(last) if matches!(last.kind, Kind::Paragraph | Kind::Text) => {
                        if !last.raw.ends_with('\n') {
                            last.raw.push('\n');
                        }
                        last.raw.push_str(raw);
                        last.text.push('\n');
                        last.text.push_str(&text);
                        let merged = last.text.clone();
                        self.set_last_job(&merged);
                    }
                    _ => {
                        let mut token = Token::new(Kind::Code, raw);
                        token.text = text;
                        tokens.push(token);
                    }
                }
                continue;
            }

            // fences
            if let Some(found) = exec(&rules.fences, rest).and_then(|c| c.get(0)) {
                position += found.as_str().len();
                tokens.push(Token::new(Kind::Code, found.as_str()));
                continue;
            }

            // heading
            if let Some(captures) = exec(&rules.heading, rest) {
                let raw = group(&captures, 0).unwrap_or_default();
                position += raw.len();
                let mut text = js_trim(group(&captures, 2).unwrap_or_default());
                if text.ends_with('#') {
                    let trimmed = rtrim(text, '#');
                    if trimmed.is_empty() || trimmed.ends_with(' ') {
                        text = js_trim(trimmed);
                    }
                }
                let text = text.to_owned();
                self.inline(&text);
                tokens.push(Token::new(Kind::Heading, raw));
                continue;
            }

            // hr
            if let Some(found) = exec(&rules.hr, rest).and_then(|c| c.get(0)) {
                // The raw text leaves out the newlines, and marked moves on by the raw text: the
                // newlines that follow are a space token, which adds one to the rule's raw text.
                let raw = rtrim(found.as_str(), '\n');
                position += raw.len();
                tokens.push(Token::new(Kind::Hr, raw));
                continue;
            }

            // blockquote
            if let Some(token) = self.blockquote(rest) {
                position += token.raw.len().max(1);
                tokens.push(token);
                continue;
            }

            // list
            if let Some(raw) = self.list(rest) {
                position += raw.len().max(1);
                tokens.push(Token::new(Kind::List, &raw));
                continue;
            }

            // html
            if let Some(found) = exec(&rules.html, rest).and_then(|c| c.get(0)) {
                position += found.as_str().len();
                tokens.push(Token::new(Kind::Html, found.as_str()));
                continue;
            }

            // def
            if let Some(captures) = exec(&rules.def, rest) {
                let raw = group(&captures, 0).unwrap_or_default();
                position += raw.len();
                let tag = collapse_spaces(&group(&captures, 1).unwrap_or_default().to_lowercase());
                match tokens.last_mut() {
                    Some(last) if matches!(last.kind, Kind::Paragraph | Kind::Text) => {
                        if !last.raw.ends_with('\n') {
                            last.raw.push('\n');
                        }
                        last.raw.push_str(raw);
                        last.text.push('\n');
                        last.text.push_str(raw);
                        let merged = last.text.clone();
                        self.set_last_job(&merged);
                    }
                    _ => {
                        if self.links.insert(tag) {
                            tokens.push(Token::new(Kind::Def, raw));
                        }
                    }
                }
                continue;
            }

            // table
            if let Some(raw) = self.table(rest) {
                position += raw.len().max(1);
                tokens.push(Token::new(Kind::Table, &raw));
                continue;
            }

            // lheading
            if let Some(captures) = exec(&rules.lheading, rest) {
                let raw = group(&captures, 0).unwrap_or_default();
                position += raw.len();
                let text = group(&captures, 1).unwrap_or_default().to_owned();
                self.inline(&text);
                tokens.push(Token::new(Kind::Heading, raw));
                continue;
            }

            // paragraph
            if self.top
                && let Some(captures) = exec(&rules.paragraph, rest)
            {
                let raw = group(&captures, 0).unwrap_or_default();
                let mut text = group(&captures, 1).unwrap_or_default();
                if text.ends_with('\n') {
                    text = &text[..text.len() - 1];
                }
                let job = self.inline(text);
                match tokens.last_mut() {
                    Some(last) if last_paragraph_clipped && last.kind == Kind::Paragraph => {
                        if !last.raw.ends_with('\n') {
                            last.raw.push('\n');
                        }
                        last.raw.push_str(raw);
                        last.text.push('\n');
                        last.text.push_str(text);
                        self.queue.pop();
                        let merged = last.text.clone();
                        self.set_last_job(&merged);
                    }
                    _ => {
                        let mut token = Token::new(Kind::Paragraph, raw);
                        token.text = text.to_owned();
                        token.job = Some(job);
                        tokens.push(token);
                    }
                }
                last_paragraph_clipped = false;
                position += raw.len().max(1);
                continue;
            }

            // text
            if let Some(found) = exec(&rules.text, rest).and_then(|c| c.get(0)) {
                let raw = found.as_str();
                position += raw.len();
                let job = self.inline(raw);
                match tokens.last_mut() {
                    Some(last) if last.kind == Kind::Text => {
                        if !last.raw.ends_with('\n') {
                            last.raw.push('\n');
                        }
                        last.raw.push_str(raw);
                        last.text.push('\n');
                        last.text.push_str(raw);
                        self.queue.pop();
                        let merged = last.text.clone();
                        self.set_last_job(&merged);
                    }
                    _ => {
                        let mut token = Token::new(Kind::Text, raw);
                        token.job = Some(job);
                        tokens.push(token);
                    }
                }
                continue;
            }

            // marked throws on a text no rule takes; such a text has no tokens here.
            break;
        }
        self.top = true;
    }

    /// The `blockquote` tokenizer: a block quote with its lazy continuations, and the nested
    /// block quote or list that a continuation joins.
    fn blockquote(&mut self, src: &str) -> Option<Token> {
        let rules = self.rules;
        let found = exec(&rules.blockquote, src)?.get(0)?;
        let mut lines: Vec<String> = rtrim(found.as_str(), '\n')
            .split('\n')
            .map(str::to_owned)
            .collect();
        let mut raw = String::new();
        let mut text = String::new();
        let mut tokens: Vec<Token> = Vec::new();

        while !lines.is_empty() {
            let mut in_blockquote = false;
            let mut current: Vec<String> = Vec::new();
            let mut index = 0;
            while index < lines.len() {
                if rules.blockquote_start.is_match(&lines[index]) {
                    current.push(lines[index].clone());
                    in_blockquote = true;
                } else if !in_blockquote {
                    current.push(lines[index].clone());
                } else {
                    break;
                }
                index += 1;
            }
            lines.drain(..index);

            let current_raw = current.join("\n");
            let current_text = replace_global(&rules.blockquote_setext, &current_raw, |captures| {
                format!("\n    {}", dollar_one(captures))
            });
            let current_text =
                replace_global(&rules.blockquote_marker, &current_text, |_| String::new());
            raw = if raw.is_empty() {
                current_raw
            } else {
                format!("{raw}\n{current_raw}")
            };
            text = if text.is_empty() {
                current_text.clone()
            } else {
                format!("{text}\n{current_text}")
            };

            let top = self.top;
            self.top = true;
            self.block_tokens(&current_text, &mut tokens, true);
            self.top = top;

            if lines.is_empty() {
                break;
            }

            let Some(last) = tokens.last().cloned() else {
                continue;
            };
            match last.kind {
                Kind::Code => break,
                Kind::Blockquote => {
                    let new_text = format!("{}\n{}", last.raw, lines.join("\n"));
                    let new_token = self.blockquote(&new_text)?;
                    raw = format!(
                        "{}{}",
                        head(&raw, raw.len().saturating_sub(last.raw.len())),
                        new_token.raw
                    );
                    text = format!(
                        "{}{}",
                        head(&text, text.len().saturating_sub(last.text.len())),
                        new_token.text
                    );
                    let at = tokens.len() - 1;
                    tokens[at] = new_token;
                    break;
                }
                Kind::List => {
                    let new_text = format!("{}\n{}", last.raw, lines.join("\n"));
                    let new_raw = self.list(&new_text)?;
                    raw = format!(
                        "{}{}",
                        head(&raw, raw.len().saturating_sub(last.raw.len())),
                        new_raw
                    );
                    text = format!(
                        "{}{}",
                        head(&text, text.len().saturating_sub(last.raw.len())),
                        new_raw
                    );
                    lines = tail(&new_text, new_raw.len())
                        .split('\n')
                        .map(str::to_owned)
                        .collect();
                    let at = tokens.len() - 1;
                    tokens[at] = Token::new(Kind::List, &new_raw);
                    continue;
                }
                _ => {}
            }
        }

        let mut token = Token::new(Kind::Blockquote, &raw);
        token.text = text;
        Some(token)
    }

    /// The `list` tokenizer: the raw text of a list. The items' contents are lexed as marked lexes
    /// them, for the link definitions and the inline jobs they add.
    fn list(&mut self, src: &str) -> Option<String> {
        let rules = self.rules;
        let captures = exec(&rules.list, src)?;
        let mut bull = js_trim(group(&captures, 1).unwrap_or_default()).to_owned();
        let ordered = bull.chars().count() > 1;
        bull = if ordered {
            format!(r"\d{{1,9}}\{}", bull.chars().last().unwrap_or('.'))
        } else {
            format!(r"\{bull}")
        };
        let item_regex = compile(&format!(r"^( {{0,3}}{bull})((?:[\t ][^\n]*)?(?:\n|$))"), "");

        let mut src = src;
        let mut list_raw = String::new();
        let mut items: Vec<String> = Vec::new();
        let mut item_raws: Vec<String> = Vec::new();
        while !src.is_empty() {
            let mut end_early = false;
            let Some(item) = exec(&item_regex, src) else {
                break;
            };
            if rules.hr.is_match(src) {
                break;
            }
            let cap0 = group(&item, 0).unwrap_or_default();
            let cap1 = group(&item, 1).unwrap_or_default();
            let cap2 = group(&item, 2).unwrap_or_default();
            let mut raw = cap0.to_owned();
            src = &src[cap0.len()..];

            let mut line = expand_leading_tabs(first_line(cap2));
            let mut next_line = first_line(src).to_owned();
            let mut blank_line = js_trim(&line).is_empty();

            let mut indent;
            let mut item_contents = String::new();
            if blank_line {
                indent = cap1.len() + 1;
            } else {
                indent = first_non_space(cap2).unwrap_or(0);
                if indent > 4 {
                    indent = 1;
                }
                item_contents = tail(&line, indent).to_owned();
                indent += cap1.len();
            }

            if blank_line && next_line.chars().all(|c| c == ' ' || c == '\t') {
                raw.push_str(&next_line);
                raw.push('\n');
                src = tail(src, next_line.len() + 1);
                end_early = true;
            }

            if !end_early {
                let item_rules = ItemRules::new(indent);
                while !src.is_empty() {
                    let raw_line = first_line(src).to_owned();
                    next_line = raw_line.clone();
                    let next_line_without_tabs = next_line.replace('\t', "    ");

                    if item_rules.fences_begin.is_match(&next_line)
                        || item_rules.heading_begin.is_match(&next_line)
                        || item_rules.html_begin.is_match(&next_line)
                        || item_rules.next_bullet.is_match(&next_line)
                        || item_rules.hr.is_match(&next_line)
                    {
                        break;
                    }

                    let search = first_non_space(&next_line_without_tabs);
                    if search.is_some_and(|found| found >= indent) || js_trim(&next_line).is_empty()
                    {
                        item_contents.push('\n');
                        item_contents.push_str(tail(&next_line_without_tabs, indent));
                    } else {
                        if blank_line {
                            break;
                        }
                        if first_non_space(&line.replace('\t', "    "))
                            .is_some_and(|found| found >= 4)
                        {
                            break;
                        }
                        if item_rules.fences_begin.is_match(&line)
                            || item_rules.heading_begin.is_match(&line)
                            || item_rules.hr.is_match(&line)
                        {
                            break;
                        }
                        item_contents.push('\n');
                        item_contents.push_str(&next_line);
                    }

                    if !blank_line && js_trim(&next_line).is_empty() {
                        blank_line = true;
                    }

                    raw.push_str(&raw_line);
                    raw.push('\n');
                    src = tail(src, raw_line.len() + 1);
                    line = tail(&next_line_without_tabs, indent).to_owned();
                }
            }

            if rules.list_is_task.is_match(&item_contents) {
                item_contents = replace_first(&rules.list_replace_task, &item_contents);
            }
            list_raw.push_str(&raw);
            item_raws.push(raw);
            items.push(item_contents);
        }

        if items.is_empty() {
            return None;
        }
        if let Some(last) = items.last_mut() {
            *last = trim_end_js(last).to_owned();
        }
        let list_raw = trim_end_js(&list_raw).to_owned();

        for item in &items {
            self.top = false;
            let mut item_tokens = Vec::new();
            self.block_tokens(item, &mut item_tokens, false);
        }
        Some(list_raw)
    }

    /// The `table` tokenizer: the raw text of a table, whose header and delimiter row have as many
    /// cells. The cells are queued for the inline lexer, as marked queues them.
    fn table(&mut self, src: &str) -> Option<String> {
        let rules = self.rules;
        let captures = exec(&rules.table, src)?;
        let delimiter = group(&captures, 2).unwrap_or_default();
        if !delimiter.contains([':', '|']) {
            return None;
        }
        let headers = split_cells(group(&captures, 1).unwrap_or_default(), None);
        let aligns_text = replace_global(&rules.table_align_chars, delimiter, |_| String::new());
        let aligns = aligns_text.split('|').count();
        let rows: Vec<String> = match group(&captures, 3) {
            Some(rows) if !js_trim(rows).is_empty() => {
                replace_first(&rules.table_row_blank_line, rows)
                    .split('\n')
                    .map(str::to_owned)
                    .collect()
            }
            _ => Vec::new(),
        };
        if headers.len() != aligns {
            return None;
        }
        let raw = group(&captures, 0).unwrap_or_default().to_owned();
        for header in &headers {
            self.inline(header);
        }
        for row in &rows {
            for cell in split_cells(row, Some(headers.len())) {
                self.inline(&cell);
            }
        }
        Some(raw)
    }

    /// `inlineTokens`: the inline tokens of a text.
    fn inline_tokens(&mut self, src: &str) -> Vec<Inline> {
        let rules = self.rules;
        let masked = self.mask(src);
        let mut tokens: Vec<Inline> = Vec::new();
        let mut position = 0;
        let mut keep_prev_char = false;
        let mut prev_char = PrevChar::None;
        while position < src.len() {
            if !keep_prev_char {
                prev_char = PrevChar::None;
            }
            keep_prev_char = false;
            let rest = &src[position..];

            // escape
            if let Some(found) = exec(&rules.escape, rest).and_then(|c| c.get(0)) {
                position += found.as_str().len();
                tokens.push(Inline::Other);
                continue;
            }

            // tag
            if let Some(found) = exec(&rules.tag, rest).and_then(|c| c.get(0)) {
                let raw = found.as_str();
                if !self.in_link && rules.start_a_tag.is_match(raw) {
                    self.in_link = true;
                } else if self.in_link && rules.end_a_tag.is_match(raw) {
                    self.in_link = false;
                }
                position += raw.len();
                tokens.push(Inline::Html(raw.to_owned()));
                continue;
            }

            // link
            if let Some(length) = self.link(rest) {
                position += length.max(1);
                tokens.push(Inline::Other);
                continue;
            }

            // reflink
            if let Some(reflink) = self.reflink(rest) {
                match reflink {
                    Reflink::Text(text) => {
                        position += text.len();
                        push_text(&mut tokens, &text);
                    }
                    Reflink::Link(length) => {
                        position += length.max(1);
                        tokens.push(Inline::Other);
                    }
                }
                continue;
            }

            // emStrong
            if let Some((length, token)) = self.em_strong(rest, &masked, prev_char) {
                position += length.max(1);
                tokens.push(token);
                continue;
            }

            // codespan
            if let Some(found) = exec(&rules.code_span, rest).and_then(|c| c.get(0)) {
                position += found.as_str().len();
                tokens.push(Inline::Other);
                continue;
            }

            // br
            if let Some(found) = exec(&rules.br, rest).and_then(|c| c.get(0)) {
                position += found.as_str().len();
                tokens.push(Inline::Other);
                continue;
            }

            // del
            if let Some(captures) = exec(&rules.del, rest) {
                let raw = group(&captures, 0).unwrap_or_default();
                let text = group(&captures, 2).unwrap_or_default().to_owned();
                position += raw.len().max(1);
                self.inline_tokens(&text);
                tokens.push(Inline::Other);
                continue;
            }

            // autolink
            if let Some(found) = exec(&rules.autolink, rest).and_then(|c| c.get(0)) {
                position += found.as_str().len();
                tokens.push(Inline::Other);
                continue;
            }

            // url
            if !self.in_link
                && let Some(length) = self.url(rest)
            {
                position += length;
                tokens.push(Inline::Other);
                continue;
            }

            // inlineText
            if let Some(found) = exec(&rules.inline_text, rest).and_then(|c| c.get(0)) {
                let raw = found.as_str();
                position += raw.len().max(1);
                if let Some(last) = raw.chars().last()
                    && last != '_'
                {
                    prev_char = if (last as u32) > 0xFFFF {
                        PrevChar::Surrogate
                    } else {
                        PrevChar::Char(last)
                    };
                }
                keep_prev_char = true;
                push_text(&mut tokens, raw);
                continue;
            }

            // marked throws on a text no rule takes; such a text has no more tokens here.
            break;
        }
        tokens
    }

    /// The masked text of `inlineTokens`: the reference links that are defined, the escaped
    /// punctuation, and the links, code spans and tags that emphasis skips over, each replaced by
    /// characters that delimiter runs do not meet. The fillers have marked's lengths in UTF-16
    /// units, and each search goes on at the unit after its match, as `lastIndex` does: an escaped
    /// symbol outside the Basic Multilingual Plane makes the text one unit shorter there.
    fn mask(&self, src: &str) -> String {
        let rules = self.rules;
        let mut masked = src.to_owned();
        if !self.links.is_empty() {
            let mut position = 0;
            while let Some(found) = rules.reflink_search.find_at(&masked, position) {
                let (start, end) = (found.start(), found.end());
                let text = found.as_str().to_owned();
                let next_unit = units(&masked[..end]).max(units(&masked[..start]) + 1);
                let label = text
                    .rfind('[')
                    .map(|open| &text[open + 1..text.len().saturating_sub(1).max(open + 1)])
                    .unwrap_or_default();
                if self.links.contains(label) {
                    let filler = format!("[{}]", "a".repeat(units(&text).saturating_sub(2)));
                    masked.replace_range(start..end, &filler);
                }
                position = byte_at_unit(&masked, next_unit, true);
                if position >= masked.len() {
                    break;
                }
            }
        }
        let mut position = 0;
        while let Some(found) = rules.any_punctuation.find_at(&masked, position) {
            let (start, end) = (found.start(), found.end());
            let next_unit = units(&masked[..end]);
            masked.replace_range(start..end, "++");
            position = byte_at_unit(&masked, next_unit, true);
            if position >= masked.len() {
                break;
            }
        }
        let mut position = 0;
        while let Some(found) = rules.block_skip.find_at(&masked, position) {
            let (start, end) = (found.start(), found.end());
            let next_unit = units(&masked[..end]).max(units(&masked[..start]) + 1);
            let filler = format!("[{}]", "a".repeat(units(found.as_str()).saturating_sub(2)));
            masked.replace_range(start..end, &filler);
            position = byte_at_unit(&masked, next_unit, true);
            if position >= masked.len() {
                break;
            }
        }
        masked
    }

    /// The `link` tokenizer: the length of an inline link or image at the start of the text.
    fn link(&mut self, src: &str) -> Option<usize> {
        let rules = self.rules;
        let captures = exec(&rules.link, src)?;
        let mut cap0 = group(&captures, 0).unwrap_or_default();
        let cap1 = group(&captures, 1).unwrap_or_default();
        let cap2 = group(&captures, 2).unwrap_or_default();
        let trimmed_url = js_trim(cap2);
        if trimmed_url.starts_with('<') {
            if !trimmed_url.ends_with('>') {
                return None;
            }
            let without_bracket = &trimmed_url[..trimmed_url.len() - 1];
            let slashes = without_bracket.len() - rtrim(without_bracket, '\\').len();
            if (slashes + 1).is_multiple_of(2) {
                return None;
            }
        } else {
            match find_closing_bracket(cap2) {
                Err(()) => return None,
                Ok(Some(last_paren)) => {
                    let start = if cap0.starts_with('!') { 5 } else { 4 };
                    let link_length = start + cap1.len() + last_paren;
                    cap0 = js_trim(head(cap0, link_length));
                }
                Ok(None) => {}
            }
        }
        let text = cap1.replace("\\[", "[").replace("\\]", "]");
        self.output_link(&text);
        Some(cap0.len())
    }

    /// The `reflink` tokenizer: a reference link that is defined, or the first character of one
    /// that is not, as text.
    fn reflink(&mut self, src: &str) -> Option<Reflink> {
        let rules = self.rules;
        let captures = exec(&rules.reflink, src).or_else(|| exec(&rules.nolink, src))?;
        let cap0 = group(&captures, 0).unwrap_or_default();
        let label = group(&captures, 2)
            .filter(|label| !label.is_empty())
            .or_else(|| group(&captures, 1))
            .unwrap_or_default();
        let link_string = collapse_spaces(label).to_lowercase();
        if !self.links.contains(&link_string) {
            let first = cap0.chars().next().map(String::from).unwrap_or_default();
            return Some(Reflink::Text(first));
        }
        let text = group(&captures, 1)
            .unwrap_or_default()
            .replace("\\[", "[")
            .replace("\\]", "]");
        self.output_link(&text);
        Some(Reflink::Link(cap0.len()))
    }

    /// `outputLink`: the link's text is lexed inside the link, for the state it leaves.
    fn output_link(&mut self, text: &str) {
        self.in_link = true;
        self.inline_tokens(text);
        self.in_link = false;
    }

    /// The `url` tokenizer: the length of a bare URL or e-mail address at the start of the text.
    fn url(&self, src: &str) -> Option<usize> {
        let rules = self.rules;
        let captures = exec(&rules.url, src)?;
        let cap0 = group(&captures, 0).unwrap_or_default();
        if group(&captures, 2) == Some("@") {
            return (!cap0.is_empty()).then_some(cap0.len());
        }
        let mut current = cap0.to_owned();
        loop {
            let previous = current.clone();
            current = rules
                .backpedal
                .find(&previous)
                .map(|found| found.as_str().to_owned())
                .unwrap_or_default();
            if current == previous {
                break;
            }
        }
        (!current.is_empty()).then_some(current.len())
    }

    /// The `emStrong` tokenizer: the length of an emphasis or strong emphasis at the start of the
    /// text, and its token. `masked` is the whole masked text, which marked slices from its end by
    /// the length of the text, in UTF-16 units, as are the positions it adds up.
    fn em_strong(
        &mut self,
        src: &str,
        masked: &str,
        prev_char: PrevChar,
    ) -> Option<(usize, Inline)> {
        let rules = self.rules;
        let left = exec(&rules.em_strong_l_delim, src)?;
        let left_text = group(&left, 0).unwrap_or_default();
        let prev_is_alphanumeric = match prev_char {
            PrevChar::Char(character) => {
                rules.unicode_alpha_numeric.is_match(&character.to_string())
            }
            PrevChar::None | PrevChar::Surrogate => false,
        };
        if group(&left, 3).is_some() && prev_is_alphanumeric {
            return None;
        }
        let next_char = group(&left, 1)
            .or_else(|| group(&left, 2))
            .unwrap_or_default();
        let prev_is_punctuation = match prev_char {
            PrevChar::Char(character) => rules.punctuation.is_match(&character.to_string()),
            PrevChar::None | PrevChar::Surrogate => false,
        };
        if !(next_char.is_empty() || matches!(prev_char, PrevChar::None) || prev_is_punctuation) {
            return None;
        }

        let left_length = left_text.chars().count() - 1;
        let end_regex = if left_text.starts_with('*') {
            &rules.em_strong_r_delim_ast
        } else {
            &rules.em_strong_r_delim_und
        };
        // `maskedSrc.slice(-1 * src.length + lLength)`. A start inside a surrogate pair leaves a
        // lone low surrogate first in marked's text: the search here starts after the pair, and
        // the positions count that unit.
        let start_unit = (units(masked) + left_length).saturating_sub(units(src));
        let hay_start = byte_at_unit(masked, start_unit, true);
        let offset = units(&masked[..hay_start]) - start_unit;
        let hay = &masked[hay_start..];
        let mut delim_total = left_length as i64;
        let mut mid_delim_total = 0i64;
        let mut position = 0;
        while let Some(right) = end_regex.captures_at(hay, position) {
            let whole = right.get(0)?;
            position = if whole.end() > whole.start() {
                whole.end()
            } else {
                next_boundary(hay, whole.end())
            };
            let groups: Vec<bool> = (1..=6).map(|index| right.get(index).is_some()).collect();
            let Some(right_delim) = (1..=6).find_map(|index| group(&right, index)) else {
                if position > hay.len() {
                    break;
                }
                continue;
            };
            let mut right_length = right_delim.chars().count() as i64;
            if groups[2] || groups[3] {
                delim_total += right_length;
                continue;
            } else if (groups[4] || groups[5])
                && !left_length.is_multiple_of(3)
                && (left_length as i64 + right_length) % 3 == 0
            {
                mid_delim_total += right_length;
                continue;
            }
            delim_total -= right_length;
            if delim_total > 0 {
                continue;
            }
            right_length = right_length.min(right_length + delim_total + mid_delim_total);
            let index = offset + units(&hay[..whole.start()]);
            let last_char_units = whole.as_str().chars().next().map_or(0, char::len_utf16);
            let raw_units = left_length + index + last_char_units + right_length.max(0) as usize;
            let raw = &src[..byte_at_unit(src, raw_units, false)];
            let raw_unit_length = units(raw);
            if (left_length as i64).min(right_length) % 2 == 1 {
                let end = byte_at_unit(raw, raw_unit_length.saturating_sub(1), false);
                let text = raw.get(1..end.max(1)).unwrap_or_default().to_owned();
                let inner = self.inline_tokens(&text);
                return Some((raw.len(), Inline::Em(inner)));
            }
            let end = byte_at_unit(raw, raw_unit_length.saturating_sub(2), false);
            let text = raw.get(2..end.max(2)).unwrap_or_default().to_owned();
            let inner = self.inline_tokens(&text);
            return Some((raw.len(), Inline::Strong(inner)));
        }
        None
    }
}

/// What the `reflink` tokenizer gives: text, or the length of a link.
enum Reflink {
    Text(String),
    Link(usize),
}

/// A text token, merged into the text token before it as marked merges them.
fn push_text(tokens: &mut Vec<Inline>, text: &str) {
    if let Some(Inline::Text(last)) = tokens.last_mut() {
        last.push_str(text);
    } else {
        tokens.push(Inline::Text(text.to_owned()));
    }
}

/// `replace(/\s+/g, ' ')`.
fn collapse_spaces(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_space = false;
    for character in text.chars() {
        if is_js_space(character) {
            if !in_space {
                out.push(' ');
            }
            in_space = true;
        } else {
            out.push(character);
            in_space = false;
        }
    }
    out
}

/// `replace(/^\t+/, (t) => ' '.repeat(3 * t.length))`.
fn expand_leading_tabs(line: &str) -> String {
    let tabs = line.bytes().take_while(|byte| *byte == b'\t').count();
    format!("{}{}", " ".repeat(3 * tabs), &line[tabs..])
}

/// `trimEnd`.
fn trim_end_js(text: &str) -> &str {
    text.trim_end_matches(is_js_space)
}

/// The first match of a pattern replaced by nothing, as `String.replace` with a pattern that is not
/// global does.
fn replace_first(regex: &Regex, text: &str) -> String {
    match regex.find(text) {
        Some(found) => format!("{}{}", &text[..found.start()], &text[found.end()..]),
        None => text.to_owned(),
    }
}

/// The length of a text in UTF-16 code units, which marked's positions count.
fn units(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

/// The byte index of a position in UTF-16 units. A position inside a surrogate pair gives the end
/// of its character with `ceil`, and its start without.
fn byte_at_unit(text: &str, unit: usize, ceil: bool) -> usize {
    let mut seen = 0;
    for (index, character) in text.char_indices() {
        if seen >= unit {
            return index;
        }
        let next = seen + character.len_utf16();
        if next > unit {
            return if ceil {
                index + character.len_utf8()
            } else {
                index
            };
        }
        seen = next;
    }
    text.len()
}

/// The byte index after the character at `index`, or past the end.
fn next_boundary(text: &str, index: usize) -> usize {
    text[index.min(text.len())..]
        .chars()
        .next()
        .map_or(text.len() + 1, |character| index + character.len_utf8())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rule_compiles() {
        LazyLock::force(&RULES);
        for indent in 1..=6 {
            ItemRules::new(indent);
        }
    }

    #[test]
    fn javascript_patterns_keep_their_meaning() {
        assert_eq!(js("^a$", ""), r"\Aa\z");
        assert_eq!(js("^a$", "gm"), "^a$");
        assert_eq!(js("[a-z-_]", ""), r"[a-z\-_]");
        assert_eq!(js(r"[\w:-]", ""), r"[A-Za-z0-9_:\-]");
        assert_eq!(strip_carets(r"^a[^b]?^c"), "a[^b]?c");
    }

    #[test]
    fn paragraphs_and_blocks() {
        assert_eq!(
            lex("**bold** and *it*\nnext"),
            vec![Block::Paragraph(vec![
                Inline::Strong(vec![Inline::Text("bold".into())]),
                Inline::Text(" and ".into()),
                Inline::Em(vec![Inline::Text("it".into())]),
                Inline::Text("\nnext".into()),
            ])]
        );
        assert_eq!(lex("# head"), vec![Block::Other("# head".into())]);
        assert_eq!(
            lex("x\n- item"),
            vec![
                Block::Paragraph(vec![Inline::Text("x".into())]),
                Block::Other("- item".into())
            ]
        );
    }
}
