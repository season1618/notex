use Span::*;

#[derive(Debug)]
pub enum Block {
    Header { header: Inline, level: u32, id: String },
    Blockquote { lines: Vec<Inline> },
    ListElement(List),
    Image { title: Inline, url: String },
    LinkCard { title: String, image: Option<String>, url: String, description: Option<String>, site_name: Option<String> },
    MathBlock { math: String },
    CodeBlock { lang: String, code: String },
    Table { head: Vec<Vec<Inline>>, body: Vec<Vec<Inline>> },
    Paragraph { text: Inline },
}

#[derive(Clone, Debug)]
pub struct Inline(pub Vec<Span>);

#[derive(Clone, Debug)]
pub enum Span {
    Link { text: Inline, url: String },
    Bold { text: Inline },
    Ital { text: Inline },
    Math { math: String },
    Code { code: String },
    Text { text: String },
}

pub struct HtmlText<'a>(pub &'a str);

#[derive(Debug)]
pub struct List {
    pub ordered: bool,
    pub items: Vec<ListItem>,
}

#[derive(Debug)]
pub struct ListItem {
    pub item: Inline,
    pub list: List,
}

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

impl std::fmt::Display for Inline {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for item in &self.0 {
            item.fmt(f)?;
        }
        Ok(())
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
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
        for c in self.0.chars() {
            match c {
                '<' => write!(f, "&lt;")?,
                '>' => write!(f, "&gt;")?,
                c => write!(f, "{c}")?,
            }
        }
        Ok(())
    }
}
