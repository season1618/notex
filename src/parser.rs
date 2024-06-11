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
                Paragraph { spans } if spans.is_empty() => {},
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
        if self.chs.starts_with("+ ") || self.chs.starts_with("- ") || self.chs.starts_with("* ") || self.starts_with_num() {
            return ListElement(self.parse_list(0));
        }

        // image
        if self.starts_with_next("![](") {
            return self.parse_image();
        }

        // link card
        if self.starts_with_next("?[](") {
            return self.parse_link_card();
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
        return self.parse_paragraph();
    }

    fn parse_header(&mut self, level: u32) -> Block {
        let spans = self.parse_spans();
        let mut header = String::new();
        for span in &spans {
            match span {
                Link { text, .. } => { header.push_str(text); },
                Emphasis { text, .. } => { header.push_str(text); },
                Math { math } => { header.push_str(&format!("\\({}\\)", math)) },
                Code { code } => { header.push_str(code); },
                Text { text } => { header.push_str(text); },
                _ => {},
            }
        }

        let count = self.headers.insert(header.clone());
        let id = if count == 0 { format!("{}", &header) } else { format!("{}-{}", &header, count) };
        let href = format!("#{}", &id);

        // modify title or table of contents
        if level == 1 {
            self.title = header.clone();
        } else {
            let mut cur = &mut self.toc;
            for _ in 2..level {
                cur = &mut cur.items.last_mut().unwrap().list;
            }
            cur.items.push(ListItem {
                spans: vec![ Link { text: header.clone(), url: href.clone() }],
                list: List { ordered: true, items: Vec::new() },
            });
        }
        Header { spans, level, id }
    }

    fn parse_blockquote(&mut self) -> Block {
        let mut lines = Vec::new();
        while self.starts_with_next("> ") {
            lines.push(self.parse_spans());
        }
        Blockquote { lines }
    }

    fn parse_list(&mut self, min_indent: usize) -> List {
        let mut ordered = false;
        let mut items = Vec::new();
        while !self.chs.is_empty() {
            let mut indent = 0;
            let mut chs = self.chs;
            while let Some((c, rest)) = uncons(chs) {
                if c == ' ' {
                    chs = rest;
                    indent += 1;
                } else {
                    break;
                }
            }

            if min_indent <= indent {
                self.chs = chs;

                if self.starts_with_next("+ ") || self.starts_with_next("- ") || self.starts_with_next("* ") {
                    ordered = false;
                    items.push(ListItem {
                        spans: self.parse_spans(),
                        list: self.parse_list(indent + 1),
                    });
                    continue;
                }

                if self.starts_with_num_next() {
                    ordered = true;
                    items.push(ListItem {
                        spans: self.parse_spans(),
                        list: self.parse_list(indent + 1),
                    });
                    continue;
                }
            }
            break;
        }
        List { ordered, items }
    }

    fn parse_image(&mut self) -> Block {
        let mut url = String::new();
        while let Some(c) = self.next_char_until(")") {
            url.push(c);
        }
        Image { url }
    }

    fn parse_link_card(&mut self) -> Block {
        let mut url = String::new();
        while let Some(c) = self.next_char_until(")") {
            url.push(c);
        }
        let (title, image, description, site_name) = get_ogp_info(&url);
        LinkCard { title, image, url, description, site_name }
    }

    fn parse_math_block(&mut self) -> Block {
        let mut math = String::new();
        while let Some(c) = self.next_char_until("$$") {
            math.push_str(&self.escape(c));
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
            code.push_str(&self.escape(c));
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
            while let Some(c) = self.next_char_except("|\r\n") {
                data.push_str(&self.escape(c));
            }
            row.push(data);
            self.starts_with_next("|");
        }
        if row.iter().all(|s| s.chars().all(|c| c == '-' || c == ' ')) {
            return None;
        }
        Some(row)
    }

    fn parse_paragraph(&mut self) -> Block {
        Paragraph { spans: self.parse_spans() }
    }

    fn parse_spans(&mut self) -> Vec<Span> {
        let mut spans = Vec::new();
        while !self.chs.is_empty() && !self.starts_with_newline_next() {
            // link
            if self.starts_with_next("[") {
                spans.push(self.parse_link());
                continue;
            }

            // strong
            if self.starts_with_next("**") {
                spans.push(self.parse_strong('*'));
                continue;
            }
            if self.starts_with_next("__") {
                spans.push(self.parse_strong('_'));
                continue;
            }

            // emphasis
            if self.starts_with_next("*") {
                spans.push(self.parse_emphasis('*'));
                continue;
            }
            if self.starts_with_next("_") {
                spans.push(self.parse_emphasis('_'));
                continue;
            }

            // math
            if self.starts_with_next("$") {
                spans.push(self.parse_math());
                continue;
            }

            // code
            if self.starts_with_next("`") {
                spans.push(self.parse_code());
                continue;
            }

            // text
            spans.push(self.parse_text());
        }
        spans
    }

    fn parse_link(&mut self) -> Span {
        let mut text = String::new();
        let mut url = String::new();
        let mut chs = self.chs;

        loop {
            match uncons_except_newline(chs) {
                Some((']', rest)) => { chs = rest; break; },
                Some((c, rest)) => { chs = rest; text.push_str(&self.escape(c)); },
                None => { return Text { text: String::from("[") }; },
            }
        }

        match uncons_except_newline(chs) {
            Some(('(', rest)) => { chs = rest; },
            _ => { return Text { text: String::from("[") }; },
        }

        loop {
            match uncons_except_newline(chs) {
                Some((')', rest)) => { chs = rest; break; },
                Some((c, rest)) => { chs = rest; url.push(c); },
                None => { return Text { text: String::from("[") }; },
            }
        }

        self.chs = chs;

        if text.is_empty() {
            text = get_title(&url);
        }

        Link { text, url }
    }

    fn parse_strong(&mut self, d: char) -> Span {
        let mut text = String::new();
        let mut chs = self.chs;
        while let Some((c, rest)) = uncons_except_newline(chs) {
            if c == d {
                self.chs = rest;
                if self.starts_with_next(&d.to_string()) {
                    return Strong { text };
                } else {
                    return Emphasis { text };
                }
            }
            chs = rest;
            text.push_str(&self.escape(c));
        }
        Text { text: format!("{0}{0}", d) }
    }

    fn parse_emphasis(&mut self, d: char) -> Span {
        let mut text = String::new();
        let mut chs = self.chs;
        while let Some((c, rest)) = uncons_except_newline(chs) {
            if c == d {
                self.chs = rest;
                return Emphasis { text };
            }
            chs = rest;
            text.push_str(&self.escape(c));
        }
        Text { text: d.to_string() }
    }

    fn parse_math(&mut self) -> Span {
        let mut math = String::new();
        let mut chs = self.chs;
        while let Some((c, rest)) = uncons_except_newline(chs) {
            if c == '$' {
                self.chs = rest;
                return Math { math };
            }
            chs = rest;
            math.push_str(&self.escape(c));
        }
        Text { text: String::from("$") }
    }

    fn parse_code(&mut self) -> Span {
        let mut code = String::new();
        let mut chs = self.chs;
        while let Some((c, rest)) = uncons_except_newline(chs) {
            if c == '`' {
                self.chs = rest;
                return Code { code };
            }
            chs = rest;
            code.push_str(&self.escape(c));
        }
        Text { text: String::from("`") }
    }

    fn parse_text(&mut self) -> Span {
        let mut text = String::new();
        while let Some(c) = self.next_char_except("[*_$`\r\n") {
            text.push_str(&self.escape(c));
        }
        Text { text }
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

    fn next_char_except(&mut self, except: &str) -> Option<char> {
        if let Some(c) = self.chs.chars().nth(0) {
            if !except.contains(c) {
                let i = if let Some((i, _)) = self.chs.char_indices().nth(1) { i } else { self.chs.len() };
                self.chs = &self.chs[i..];
                return Some(c);
            }
        }
        None
    }

    fn starts_with_num(&self) -> bool {
        let chs = self.chs.trim_start_matches(|c: char| c.is_ascii_digit());
        chs.strip_prefix(". ").is_some()
    }

    fn starts_with_num_next(&mut self) -> bool {
        let chs = self.chs.trim_start_matches(|c: char| c.is_ascii_digit());
        if let Some(chs) = chs.strip_prefix(". ") {
            self.chs = chs;
            true
        } else {
            false
        }
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

    fn escape(&self, c: char) -> String {
        match c {
            '<' => String::from("&lt;"),
            '>' => String::from("&gt;"),
            _ => c.to_string(),
        }
    }
}

fn uncons<'a>(chs: &'a str) -> Option<(char, &'a str)> {
    if let Some(c) = chs.chars().nth(0) {
        let i = if let Some((i, _)) = chs.char_indices().nth(1) { i } else { chs.len() };
        return Some((c, &chs[i..]));
    }
    None
}

fn uncons_except<'a>(chs: &'a str, except: &str) -> Option<(char, &'a str)> {
    if let Some(c) = chs.chars().nth(0) {
        if !except.contains(c) {
            let i = if let Some((i, _)) = chs.char_indices().nth(1) { i } else { chs.len() };
            return Some((c, &chs[i..]));
        }
    }
    None
}

fn uncons_except_newline<'a>(chs: &'a str) -> Option<(char, &'a str)> {
    uncons_except(chs, "\r\n")
}

#[tokio::main]
async fn get_title(url: &String) -> String {
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
async fn get_ogp_info(url: &String) -> (String, Option<String>, Option<String>, Option<String>) {
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