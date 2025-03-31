use tokio;
use regex::Regex;
use reqwest::{self, header};

use crate::data::*;
use crate::multiset::MultiSet;
use Block::*;
use Span::*;
use SyntaxError::*;

pub fn parse(doc: &str) -> (String, List, Vec<Block>) {
    let mut parser = Parser::new(doc);
    parser.parse_document();
    return (parser.title, parser.toc, parser.content);
}

pub struct Parser<'a> {
    chs: &'a str,
    headers: MultiSet<String>,
    notes: Vec<(Inline<'a>, usize)>, note_id: usize,
    title: String,
    toc: List<'a>,
    content: Vec<Block<'a>>,
}

impl<'a> Parser<'a> {
    fn new(doc: &'a str) -> Self {
        Parser {
            chs: doc,
            headers: MultiSet::new(),
            notes: Vec::new(), note_id: 0,
            title: String::new(),
            toc: List { ordered: true, items: Vec::new() },
            content: Vec::new(),
        }
    }

    pub fn parse_document(&mut self) {
        while !self.chs.is_empty() {
            let block = self.parse_block();
            match block {
                Paragraph { text } if text.0.is_empty() => {},
                _ => { self.content.push(block); },
            }
        }

        let refs = self.catch_refs();
        self.content.push(refs);
    }

    fn parse_block(&mut self) -> Block<'a> {
        // header
        if self.starts_with_next("# ") {
            return self.parse_header(1);
        }
        if self.starts_with_next("## ") {
            return self.parse_header(2);
        }
        if self.starts_with_next("### ") {
            return self.parse_header(3);
        }
        if self.starts_with_next("#### ") {
            return self.parse_header(4);
        }
        if self.starts_with_next("##### ") {
            return self.parse_header(5);
        }
        if self.starts_with_next("###### ") {
            return self.parse_header(6);
        }

        // blockquote
        if self.starts_with_next(">>") {
            return self.parse_blockquote();
        }

        // list
        if self.chs.starts_with("+ ") || self.chs.starts_with("- ") {
            return ListBlock(self.parse_list(0));
        }

        // embed
        if self.starts_with_next("@[") {
            return self.parse_embed().unwrap();
        }

        // table
        if self.chs.starts_with("|") {
            return self.parse_table().unwrap();
        }

        // math block
        if self.starts_with_next("$$") {
            return self.parse_math_block();
        }

        // code block
        if self.starts_with_next("```") {
            return self.parse_code_block();
        }

        // reference
        if self.starts_with_next("[^]") {
            return self.catch_refs();
        }

        // paragraph
        self.parse_paragraph()
    }

    fn parse_header(&mut self, level: u32) -> Block<'a> {
        let header = self.parse_inline();

        let mut header_toc = Vec::new();
        for span in &header.0 {
            match span {
                Cite { .. } => {},
                Link { text, .. } => {
                    for span in &text.0 {
                        header_toc.push(span.clone());
                    }
                },
                _ => header_toc.push(span.clone()),
            }
        }

        let mut header_id = String::new();
        for span in &header_toc {
            match span {
                Math { math } => header_id.push_str(math),
                Code { code } => header_id.push_str(code),
                Text { text } => header_id.push_str(text),
                _ => {},
            }
        }

        // modify title or table of contents
        if level == 1 {
            self.title = header_id.clone();
        } else {
            let count = self.headers.insert(header_id.clone());
            if count > 0 {
                header_id = format!("{}-{}", &header_id, count);
            }

            let mut cur = &mut self.toc;
            for _ in 2..level {
                cur = &mut cur.items.last_mut().unwrap().list;
            }
            cur.items.push(ListItem {
                item: Inline(vec![ Link { text: Inline(header_toc), url: format!("#{}", &header_id).into() } ]),
                list: List { ordered: true, items: Vec::new() },
            });
        }
        Header { header, level, id: header_id }
    }

    fn parse_blockquote(&mut self) -> Block<'a> {
        let mut lines = Vec::new();
        while !self.starts_with_next("<<") {
            lines.push(self.parse_inline());
        }
        Blockquote { lines }
    }

    fn parse_list(&mut self, min_indent: usize) -> List<'a> {
        let mut ordered = false;
        let mut items = Vec::new();
        while !self.chs.is_empty() {
            let chs = self.chs.trim_start_matches(' ');
            let indent = self.chs.len() - chs.len();

            if min_indent <= indent {
                self.chs = chs;

                if self.starts_with_next("- ") {
                    ordered = false;
                    items.push(ListItem {
                        item: self.parse_inline(),
                        list: self.parse_list(indent + 1),
                    });
                    continue;
                }

                if self.starts_with_next("+ ") {
                    ordered = true;
                    items.push(ListItem {
                        item: self.parse_inline(),
                        list: self.parse_list(indent + 1),
                    });
                    continue;
                }
            }
            break;
        }
        List { ordered, items }
    }

    fn parse_embed(&mut self) -> Result<Block<'a>, SyntaxError> {
        let text = self.parse_until_trim(Self::parse_cite, &["]("])?;
        let url = self.read_until_trim(&[")"])?;

        if url.ends_with(".png") || url.ends_with(".jpg") {
            let title = Inline(text);
            Ok(Image { title, url })
        } else {
            let (title, image, description, site_name) = get_ogp_info(&url);
            Ok(LinkCard { title, image, url, description, site_name })
        }
    }

    fn parse_table(&mut self) -> Result<Block<'a>, SyntaxError> {
        let mut head = Vec::new();
        let mut body = Vec::new();
        while let Some(row) = self.parse_table_row()? {
            head.push(row);
        }
        while let Some(row) = self.parse_table_row()? {
            body.push(row);
        }
        Ok(Table { head, body })
    }

    fn parse_table_row(&mut self) -> Result<Option<Vec<Inline<'a>>>, SyntaxError> {
        if self.starts_with_next("-") {
            self.read_until_trim(&["\n", "\r\n"]).unwrap();
            return Ok(None);
        }
        if !self.starts_with_next("|") {
            return Ok(None);
        }

        let mut row: Vec<Inline<'a>> = Vec::new();
        while !self.is_eol() {
            let data = Inline(self.parse_until_trim(Self::parse_cite, &["|"])?);
            row.push(data);
        }
        Ok(Some(row))
    }

    fn parse_math_block(&mut self) -> Block<'a> {
        let math = self.read_until_trim(&["$$"]).unwrap();
        MathBlock { math }
    }

    fn parse_code_block(&mut self) -> Block<'a> {
        let lang = self.read_until_trim(&["\n", "\r\n"]).unwrap();
        let code = self.read_until_trim(&["```"]).unwrap();
        CodeBlock { lang, code }
    }

    fn parse_paragraph(&mut self) -> Block<'a> {
        Paragraph { text: self.parse_inline() }
    }

    fn catch_refs(&mut self) -> Block<'a> {
        let mut refs = Vec::new();
        refs.append(&mut self.notes);

        Ref(refs)
    }

    fn parse_inline(&mut self) -> Inline<'a> {
        let mut text = Vec::new();
        while !self.is_eol() {
            text.push(self.parse_cite().unwrap());
        }
        Inline(text)
    }

    fn parse_cite(&mut self) -> Result<Span<'a>, SyntaxError> {
        if self.starts_with_next("[^") {
            self.note_id += 1;
            let note = Inline(self.parse_until_trim(Self::parse_link, &["]"])?);
            let id = self.note_id;

            self.notes.push((note, id));

            Ok(Cite { id })
        } else {
            self.parse_link()
        }
    }

    fn parse_link(&mut self) -> Result<Span<'a>, SyntaxError> {
        if self.starts_with_next("[") { // link
            let text = self.parse_until_trim(Self::parse_emph, &["]("])?;
            let url: std::borrow::Cow<'a, str> = self.read_until_trim(&[")"])?.into();

            let text = if text.is_empty() {
                Inline(vec![ Text { text: get_title(url.as_ref()).into() } ])
            } else { Inline(text) };

            Ok(Link { text, url })
        } else {
            self.parse_emph()
        }
    }

    fn parse_emph(&mut self) -> Result<Span<'a>, SyntaxError> {
        if self.starts_with_next("**") {
            let text = Inline(self.parse_until_trim(Self::parse_emph, &["**"])?);
            Ok(Bold { text })
        } else if self.starts_with_next("__") {
            let text = Inline(self.parse_until_trim(Self::parse_emph, &["__"])?);
            Ok(Ital { text })
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Result<Span<'a>, SyntaxError> {
        // math
        if self.starts_with_next("$") {
            let math = self.read_until_trim(&["$"])?;
            return Ok(Math { math });
        }

        // code
        if self.starts_with_next("`") {
            let code = self.read_until_trim(&["`"])?;
            return Ok(Code { code });
        }

        // text
        let text = self.read_until(&["|", "**", "__", "[", "]", "$", "`", "\n", "\r\n"]).into();
        Ok(Text { text })
    }

    fn read_until(&mut self, terms: &[&str]) -> &'a str {
        let mut chs = self.chs.chars();
        let mut start = self.chs.len();
        while !chs.as_str().is_empty() {
            if chs.as_str().starts_with("\\") {
                chs.next();
                chs.next();
                continue;
            }
            if terms.iter().any(|&term| chs.as_str().starts_with(term)) {
                let rest = chs.as_str();
                start -= rest.len();
                break;
            }
            chs.next();
        }
        let text = &self.chs[..start];
        self.chs = &self.chs[start..];
        text
    }

    fn read_until_trim(&mut self, terms: &'static [&str]) -> Result<&'a str, SyntaxError> {
        let mut chs = self.chs.chars();
        let mut start = self.chs.len();
        let mut end = self.chs.len();
        while !chs.as_str().is_empty() {
            if chs.as_str().starts_with("\\") {
                chs.next();
                chs.next();
                continue;
            }
            if let Some(&term) = terms.iter().find(|&term| chs.as_str().starts_with(term)) {
                let rest = chs.as_str();
                start -= rest.len();
                let rest = rest.strip_prefix(term).unwrap();
                end -= rest.len();

                let text = &self.chs[..start];
                self.chs = &self.chs[end..];
                return Ok(text);
            }
            chs.next();
        }
        
        Err(Expect(terms))
    }

    fn parse_until_trim<T>(&mut self, mut parser: impl FnMut(&mut Self) -> Result<T, SyntaxError>, terms: &'static [&str]) -> Result<Vec<T>, SyntaxError> {
        let mut res = Vec::new();
        while !self.chs.is_empty() {
            if let Some(term) = terms.iter().find(|&term| self.chs.starts_with(term)) {
                self.chs = self.chs.strip_prefix(term).unwrap();
                return Ok(res);
            }
            res.push(parser(self)?);
        }

        Err(Expect(terms))
    }

    fn starts_with_next(&mut self, prefix: &str) -> bool {
        if let Some(chs) = self.chs.strip_prefix(prefix) {
            self.chs = chs;
            true
        } else {
            false
        }
    }

    fn is_eol(&mut self) -> bool {
        self.chs.is_empty() || self.starts_with_next("\n") || self.starts_with_next("\r\n")
    }
}

#[tokio::main]
async fn get_title(url: &str) -> String {
    let client = reqwest::Client::new();
    let Ok(res) = client.get(url).header(header::ACCEPT, header::HeaderValue::from_str("text/html").unwrap()).send().await else {
        return String::new();
    };
    let Ok(body) = res.text().await else {
        return String::new();
    };
    let regex = Regex::new("<title>(.*)</title>").unwrap();
    if let Some(caps) = regex.captures(&body) {
        return caps[1].to_string().clone();
    }
    return String::new();
}

#[tokio::main]
async fn get_ogp_info(url: &str) -> (String, Option<String>, Option<String>, Option<String>) {
    let mut title = String::new();
    let mut image = None;
    let mut description = None;
    let mut site_name = None;

    let client = reqwest::Client::new();
    let Ok(res) = client.get(url).header(header::ACCEPT, header::HeaderValue::from_str("text/html").unwrap()).send().await else {
        return (title, image, description, site_name);
    };
    let Ok(body) = res.text().await else {
        return (title, image, description, site_name);
    };

    let regex = Regex::new("property=\"og:([^\"]*)\" content=\"([^\"]*)\"").unwrap();
    for caps in regex.captures_iter(&body) {
        match &caps[1] {
            "title" => { title = caps[2].to_string(); },
            "image" => { image = Some(caps[2].to_string()); },
            "description" => { description = Some(caps[2].to_string()); },
            "site_name" => { site_name = Some(caps[2].to_string()); },
            _ => {},
        }
    }

    if title.is_empty() {
        let regex = Regex::new("<title>(.*)</title>").unwrap();
        if let Some(caps) = regex.captures(&body) {
            title = caps[1].to_string();
        }
    }

    (title, image, description, site_name)
}