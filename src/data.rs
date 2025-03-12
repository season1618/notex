#[derive(Debug)]
pub enum Block {
    Header { header: Inline, level: u32, id: String },
    Blockquote { lines: Vec<Inline> },
    ListElement(List),
    Image { title: Inline, url: String },
    LinkCard { title: String, image: Option<String>, url: String, description: Option<String>, site_name: Option<String> },
    MathBlock { math: String },
    CodeBlock { lang: String, code: String },
    Table { head: Vec<Vec<String>>, body: Vec<Vec<String>> },
    Paragraph { spans: Inline },
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

#[derive(Debug)]
pub struct List {
    pub ordered: bool,
    pub items: Vec<ListItem>,
}

#[derive(Debug)]
pub struct ListItem {
    pub spans: Inline,
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