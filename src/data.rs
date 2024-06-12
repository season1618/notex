#[derive(Debug)]
pub enum Block {
    Header { prims: Vec<Prim>, level: u32, id: String },
    Blockquote { lines: Vec<Vec<Span>> },
    ListElement(List),
    Image { title: Vec<Prim>, url: String },
    LinkCard { title: String, image: Option<String>, url: String, description: Option<String>, site_name: Option<String> },
    MathBlock { math: String },
    CodeBlock { lang: String, code: String },
    Table { head: Vec<Vec<String>>, body: Vec<Vec<String>> },
    Paragraph { spans: Vec<Span> },
}

#[derive(Clone, Debug)]
pub enum Span {
    Bold { text: String },
    Ital { text: String },
    PrimElem(Prim),
}

#[derive(Clone, Debug)]
pub enum Prim {
    Link { text: Vec<Prim>, url: String },
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
    pub spans: Vec<Span>,
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