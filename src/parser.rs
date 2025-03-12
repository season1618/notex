use tokio;
use regex::Regex;
use reqwest::{self, header};

use crate::data::*;
use crate::multiset::MultiSet;
use Block::*;
use Span::*;

pub fn parse_markdown(doc: &str) -> (String, List, Vec<Block>) {
    let mut parser = Parser::new(doc);
    parser.parse_markdown();
    return (parser.title, parser.toc, parser.content);
}

pub struct Parser<'a> {
    chs: &'a str,
    headers: MultiSet<String>,
    title: String,
    toc: List,
    content: Vec<Block>,
}

impl<'a> Parser<'a> {
    fn new(doc: &'a str) -> Self {
        Parser {
            chs: doc,
            headers: MultiSet::new(),
            title: String::new(),
            toc: List { ordered: true, items: Vec::new() },
            content: Vec::new(),
        }
    }

    pub fn parse_markdown(&mut self) {
        while !self.chs.is_empty() {
            let block = self.parse_block();
            match block {
                Paragraph { text } if text.0.is_empty() => {},
                _ => { self.content.push(block); },
            }
        }
    }

    fn parse_block(&mut self) -> Block {
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
        if self.chs.starts_with("> ") {
            return self.parse_blockquote();
        }

        // list
        if self.chs.starts_with("+ ") || self.chs.starts_with("- ") {
            return ListElement(self.parse_list(0));
        }

        // embed
        if self.starts_with_next("@[") {
            return self.parse_embed();
        }

        // math block
        if self.starts_with_next("$$") {
            return self.parse_math_block();
        }

        // code block
        if self.starts_with_next("```") {
            return self.parse_code_block();
        }

        // table
        if self.chs.starts_with("|") {
            return self.parse_table();
        }

        // paragraph
        self.parse_paragraph()
    }

    fn parse_header(&mut self, level: u32) -> Block {
        let mut header_toc = Vec::new();
        let mut header_id = String::new();

        let header = self.parse_inline();
        for span in &header.0 {
            match span {
                Link { text, .. } => {
                    for span in &text.0 {
                        header_toc.push(span.clone());
                    }
                },
                _ => header_toc.push(span.clone()),
            }
        }

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
                item: Inline(vec![ Link { text: Inline(header_toc), url: format!("#{}", &header_id) } ]),
                list: List { ordered: true, items: Vec::new() },
            });
        }
        Header { header, level, id: header_id }
    }

    fn parse_blockquote(&mut self) -> Block {
        let mut lines = Vec::new();
        while self.starts_with_next("> ") {
            lines.push(self.parse_inline());
        }
        Blockquote { lines }
    }

    fn parse_list(&mut self, min_indent: usize) -> List {
        let mut ordered = false;
        let mut items = Vec::new();
        while !self.chs.is_empty() {
            let mut indent = 0;
            let mut chs = self.chs;
            while let Some(rest) = chs.strip_prefix(" ") {
                chs = rest;
                indent += 1;
            }

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

    fn parse_embed(&mut self) -> Block {
        let mut text = Vec::new();
        let mut url = String::new();
        while !self.starts_with_next("](") {
            text.push(self.parse_link());
        }
        while let Some(c) = self.next_char_until(")") {
            url.push(c);
        }

        if url.ends_with(".png") || url.ends_with(".jpg") {
            let title = Inline(text);
            Image { title, url }
        } else {
            let (title, image, description, site_name) = get_ogp_info(&url);
            LinkCard { title, image, url, description, site_name }
        }
    }

    fn parse_math_block(&mut self) -> Block {
        let mut math = String::new();
        while let Some(c) = self.next_char_until("$$") {
            math.push(c);
        }
        MathBlock { math }
    }

    fn parse_code_block(&mut self) -> Block {
        let mut lang = String::new();
        while let Some(c) = self.next_char_until_newline() {
            lang.push(c);
        }
        let mut code = String::new();
        while let Some(c) = self.next_char_until("```") {
            code.push(c);
        }
        CodeBlock { lang, code }
    }

    fn parse_table(&mut self) -> Block {
        let mut head = Vec::new();
        let mut body = Vec::new();
        while let Some(row) = self.parse_table_row() {
            head.push(row);
        }
        while let Some(row) = self.parse_table_row() {
            body.push(row);
        }
        Table { head, body }
    }

    fn parse_table_row(&mut self) -> Option<Vec<String>> {
        if !self.starts_with_next("|") {
            return None;
        }

        let mut row: Vec<String> = Vec::new();
        while !self.chs.is_empty() && !self.starts_with_newline_next() {
            let mut data = String::new();
            loop {
                match self.next_char() {
                    Some('|') => break,
                    Some(c)   => data.push(c),
                    None      => break,
                }
            }
            row.push(data.trim_start().trim_end().to_string());
        }
        if row.iter().all(|s| s.chars().all(|c| c == '-')) {
            return None;
        }
        Some(row)
    }

    fn parse_paragraph(&mut self) -> Block {
        Paragraph { text: self.parse_inline() }
    }

    fn parse_inline(&mut self) -> Inline {
        let mut spans = Vec::new();
        while !self.chs.is_empty() && !self.starts_with_newline_next() {
            spans.push(self.parse_link());
        }
        Inline(spans)
    }

    fn parse_link(&mut self) -> Span {
        if self.starts_with_next("[") { // link
            let mut text = Vec::new();

            while !self.starts_with_next("](") {
                text.push(self.parse_emph());
            }

            let url = self.text_until(&[")", "\n", "\r\n"]);

            if text.is_empty() {
                text = vec![ Text { text: get_title(url) } ];
            }

            Link { text: Inline(text), url: url.to_string() }
        } else {
            self.parse_emph()
        }
    }

    fn parse_emph(&mut self) -> Span {
        if self.starts_with_next("**") {
            let mut text = Vec::new();
            while !self.starts_with_next("**") {
                text.push(self.parse_emph());
            }
            Bold { text: Inline(text) }
        } else if self.starts_with_next("__") {
            let mut text = Vec::new();
            while !self.starts_with_next("__") {
                text.push(self.parse_emph());
            }
            Ital { text: Inline(text) }
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Span {
        // math
        if self.starts_with_next("$") {
            return self.parse_math();
        }

        // code
        if self.starts_with_next("`") {
            return self.parse_code();
        }

        // text
        self.parse_text()
    }

    fn parse_math(&mut self) -> Span {
        let mut math = String::new();
        while let Some(c) = self.next_char_until("$") {
            math.push(c);
        }
        Math { math }
    }

    fn parse_code(&mut self) -> Span {
        let mut code = String::new();
        while let Some(c) = self.next_char_until("`") {
            code.push(c);
        }
        Code { code }
    }

    fn parse_text(&mut self) -> Span {
        let mut text = String::new();
        loop {
            if ["**", "__", "[", "]", "$", "`", "\n", "\r\n"].iter().any(|prefix| self.chs.starts_with(prefix)) {
                break Text { text }
            }
            if let Some(c) = self.next_char_until_newline() {
                text.push(c);
            } else {
                break Text { text }
            }
        }
    }

    fn text_until(&mut self, terms: &[&str]) -> &str {
        let mut chs = self.chs.chars();
        let mut idx = 0;
        while !chs.as_str().is_empty() {
            if let Some(&term) = terms.iter().find(|&term| chs.as_str().starts_with(term)) {
                chs = chs.as_str().trim_start_matches(term).chars();
                break;
            }
            idx += 1;
            chs.next();
        }
        let text = &self.chs[..idx];
        self.chs = chs.as_str();
        text
    }

    fn next_char(&mut self) -> Option<char> {
        let mut chs = self.chs.chars();
        if let Some(c) = chs.next() {
            self.chs = chs.as_str();
            Some(c)
        } else {
            None
        }
    }

    fn next_char_until(&mut self, until: &str) -> Option<char> {
        if self.chs.starts_with(until) {
            let len = until.chars().count();
            self.chs = &self.chs[len..];
            return None;
        }
        if let Some(c) = self.chs.chars().nth(0) {
            let i = if let Some((i, _)) = self.chs.char_indices().nth(1) { i } else { self.chs.len() };
            self.chs = &self.chs[i..];
            return Some(c);
        }
        None
    }

    fn next_char_until_newline(&mut self) -> Option<char> {
        if self.chs.starts_with("\n") {
            self.chs = &self.chs[1..];
            return None;
        }
        if self.chs.starts_with("\r\n") {
            self.chs = &self.chs[2..];
            return None;
        }
        if let Some(c) = self.chs.chars().nth(0) {
            let i = if let Some((i, _)) = self.chs.char_indices().nth(1) { i } else { self.chs.len() };
            self.chs = &self.chs[i..];
            return Some(c);
        }
        None
    }

    fn starts_with_next(&mut self, prefix: &str) -> bool {
        if let Some(chs) = self.chs.strip_prefix(prefix) {
            self.chs = chs;
            true
        } else {
            false
        }
    }

    fn starts_with_newline_next(&mut self) -> bool {
        if let Some(chs) = self.chs.strip_prefix("\n") {
            self.chs = chs;
            true
        } else if let Some(chs) = self.chs.strip_prefix("\r\n") {
            self.chs = chs;
            true
        } else {
            false
        }
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