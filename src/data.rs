use std::borrow::Cow;
use Span::*;

#[derive(Debug)]
pub enum Block<'a> {
    Header { header: Inline<'a>, level: u32, id: String },
    Blockquote { lines: Vec<Inline<'a>> },
    ListBlock(List<'a>),
    Image { title: Inline<'a>, url: &'a str },
    LinkCard { title: String, image: Option<String>, url: &'a str, description: Option<String>, site_name: Option<String> },
    Table { head: Vec<Vec<Inline<'a>>>, body: Vec<Vec<Inline<'a>>> },
    MathBlock { math: &'a str },
    CodeBlock { lang: &'a str, code: &'a str },
    Paragraph { text: Inline<'a> },
    Ref(Vec<(Inline<'a>, usize)>),
}

#[derive(Debug)]
pub struct List<'a> {
    pub ordered: bool,
    pub items: Vec<ListItem<'a>>,
}

#[derive(Debug)]
pub struct ListItem<'a> {
    pub item: Inline<'a>,
    pub list: List<'a>,
}

#[derive(Clone, Debug)]
pub struct Inline<'a>(pub Vec<Span<'a>>);

#[derive(Clone, Debug)]
pub enum Span<'a> {
    Cite { id: usize },
    Link { text: Inline<'a>, url: Cow<'a, str> },
    Bold { text: Inline<'a> },
    Ital { text: Inline<'a> },
    Math { math: &'a str },
    Code { code: &'a str },
    Text { text: Cow<'a, str> },
}

pub struct HtmlText<'a>(pub &'a str);

#[derive(Debug)]
pub enum Elem {
    Title,
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
    Toc(usize),
    Content(usize),
    Str(String),
}

impl<'a> std::fmt::Display for Inline<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for item in &self.0 {
            item.fmt(f)?;
        }
        Ok(())
    }
}

impl<'a> std::fmt::Display for Span<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Cite { id} => write!(f, "<sup id=\"cite-{id}\"><a href=\"#ref-{id}\">[{id}]</a></sup>"),
            Link { text, url } => write!(f, "<a href=\"{url}\">{text}</a>"),
            Bold { text } => write!(f, "<strong>{text}</strong>"),
            Ital { text } => write!(f, "<em>{text}</em>"),
            Math { math } => write!(f, "\\({}\\)", HtmlText(math)),
            Code { code } => write!(f, "<code>{}</code>", HtmlText(code)),
            Text { text } => write!(f, "{}", HtmlText(text)),
        }
    }
}

impl<'a> std::fmt::Display for HtmlText<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut chs = self.0.chars();
        while let Some(c) = chs.next() {
            if c == '\\' {
                if let Some(c) = chs.next() {
                    escape(c, f)?;
                }
            } else {
                escape(c, f)?;
            }
        }
        Ok(())
    }
}

fn escape(c: char, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match c {
        '<' => write!(f, "&lt;"),
        '>' => write!(f, "&gt;"),
        c => write!(f, "{c}"),
    }
}
