use std::io::{self, Write};
use std::fs::File;
use chrono::{Local, Datelike, Timelike};

use crate::data::*;

use Block::*;
use Elem::*;

pub fn gen_html(dest: &mut File, title: &String, toc: &List, content: &Vec<Block>, template: &Vec<Elem>) -> Result<(), io::Error> {
    let mut codegen = CodeGen::new(dest);
    codegen.gen_html(title, toc, content, template)
}

struct CodeGen<'a> {
    dest: &'a mut File,
}

impl<'a> CodeGen<'a> {
    fn new(dest: &'a mut File) -> Self {
        CodeGen { dest }
    }

    fn gen_html(&mut self, title: &String, toc: &List, content: &Vec<Block>, template: &Vec<Elem>) -> Result<(), io::Error> {
        let datetime = Local::now();
        for chunk in template {
            match chunk {
                Title => write!(self.dest, "{}", title)?,
                Year   => write!(self.dest, "{:04}", datetime.year())?,
                Month  => write!(self.dest, "{:02}", datetime.month())?,
                Day    => write!(self.dest, "{:02}", datetime.day())?,
                Hour   => write!(self.dest, "{:02}", datetime.hour())?,
                Minute => write!(self.dest, "{:02}", datetime.minute())?,
                Second => write!(self.dest, "{:02}", datetime.second())?,
                Toc(indent) => self.gen_toc(toc, *indent)?,
                Content(indent) => self.gen_content(content, *indent)?,
                Str(text) => write!(self.dest, "{}", text)?,
            }
        }
        Ok(())
    }

    fn gen_toc(&mut self, toc: &List, indent: usize) -> Result<(), io::Error> {
        writeln!(self.dest)?;
        self.gen_list(&toc, indent)
    }

    fn gen_content(&mut self, content: &Vec<Block>, indent: usize) -> Result<(), io::Error> {
        writeln!(self.dest)?;
        for block in content {
            match block {
                Header { header, level, id } => self.gen_header(header, level, id, indent)?,
                Blockquote { lines } => self.gen_blockquote(lines, indent)?,
                ListBlock(list) => self.gen_list(list, indent)?,
                Table { head, body } => self.gen_table(head, body, indent)?,
                Image { title, url } => self.gen_image(title, url, indent)?,
                LinkCard { title, image, url, description, site_name } => self.gen_link_card(title, image, url, description, site_name, indent)?,
                MathBlock { math } => self.gen_math_block(math, indent)?,
                CodeBlock { lang, code } => self.gen_code_block(lang, code, indent)?,
                Paragraph { text } => self.gen_paragraph(text, indent)?,
            }
        }
        Ok(())
    }

    fn gen_header(&mut self, header: &Inline, level: &u32, id: &String, indent: usize) -> Result<(), io::Error> {
        let indent = " ".repeat(indent);
        writeln!(self.dest, "{indent}<h{level} id=\"{id}\">{header}</h{level}>")
    }

    fn gen_blockquote(&mut self, lines: &Vec<Inline>, indent: usize) -> Result<(), io::Error> {
        let indent = " ".repeat(indent);
        writeln!(self.dest, "{indent}<blockquote>")?;
        for line in lines {
            writeln!(self.dest, "{indent}  <p>{line}</p>")?;
        }
        writeln!(self.dest, "{indent}</blockquote>")
    }

    fn gen_list(&mut self, list: &List, depth: usize) -> Result<(), io::Error> {
        if list.items.is_empty() {
            return Ok(());
        }

        let indent = " ".repeat(depth);
        writeln!(self.dest, "{indent}<{}>", if list.ordered { "ol" } else { "ul" })?;
        for ListItem { item, list } in &list.items {
            writeln!(self.dest, "{indent}  <li>")?;
            
            writeln!(self.dest, "{indent}    {item}")?;
            self.gen_list(list, depth + 4)?;
            
            writeln!(self.dest, "{indent}  </li>")?;
        }
        writeln!(self.dest, "{indent}</{}>", if list.ordered { "ol" } else { "ul" })
    }

    fn gen_image(&mut self, title: &Inline, url: &str, indent: usize) -> Result<(), io::Error> {
        let indent = " ".repeat(indent);
        writeln!(self.dest, "{indent}<div class=\"image\">")?;
        writeln!(self.dest, "{indent}  <img src=\"{url}\">")?;
        writeln!(self.dest, "{indent}  <p class=\"caption\">{title}</p>")?;
        writeln!(self.dest, "{indent}</div>")
    }

    fn gen_link_card(&mut self, title: &String, image: &Option<String>, url: &str, description: &Option<String>, site_name: &Option<String>, indent: usize) -> Result<(), io::Error> {
        let indent = " ".repeat(indent);

        writeln!(self.dest, "{indent}<div class=\"linkcard\"><a class=\"linkcard-link\" href=\"{url}\">")?;
        writeln!(self.dest, "{indent}  <div class=\"linkcard-text\">")?;
        writeln!(self.dest, "{indent}    <h3 class=\"linkcard-title\">{title}</h3>")?;
        if let Some(desc) = description {
            writeln!(self.dest, "{indent}    <p class=\"linkcard-description\">{desc}</p>")?;
        }
        writeln!(self.dest, "{indent}    <img class=\"linkcard-favicon\" src=\"http://www.google.com/s2/favicons?domain={url}\"><span  class=\"linkcard-sitename\">{}</span>", site_name.clone().unwrap_or(url.to_string()))?;
        writeln!(self.dest, "{indent}  </div>")?;
        if let Some(img) = image {
            writeln!(self.dest, "{indent}  <img class=\"linkcard-image\" src=\"{img}\">")?;
        }
        writeln!(self.dest, "{indent}</a></div>")
    }

    fn gen_table(&mut self, head: &Vec<Vec<Inline>>, body: &Vec<Vec<Inline>>, indent: usize) -> Result<(), io::Error> {
        let indent = " ".repeat(indent);

        writeln!(self.dest, "{indent}<table>")?;

        writeln!(self.dest, "{indent}  <thead>")?;
        for row in head {
            writeln!(self.dest, "{indent}    <tr>")?;
            for data in row {
                writeln!(self.dest, "{indent}      <td>{data}</td>")?;
            }
            writeln!(self.dest, "{indent}    </tr>")?;
        }
        writeln!(self.dest, "{indent}  </thead>")?;
        
        writeln!(self.dest, "{indent}  <tbody>")?;
        for row in body {
            writeln!(self.dest, "{indent}    <tr>")?;
            for data in row {
                writeln!(self.dest, "{indent}      <td>{data}</td>")?;
            }
            writeln!(self.dest, "{indent}    </tr>")?;
        }
        writeln!(self.dest, "{indent}  </tbody>")?;
        
        writeln!(self.dest, "{indent}</table>")
    }

    fn gen_math_block(&mut self, math: &str, indent: usize) -> Result<(), io::Error> {
        let indent = " ".repeat(indent);
        writeln!(self.dest, "{indent}<p>\\[{}\\]</p>", HtmlText(math))
    }

    fn gen_code_block(&mut self, lang: &str, code: &str, indent: usize) -> Result<(), io::Error> {
        let indent = " ".repeat(indent);
        let lang = if lang == "" { "plaintext" } else { lang };
        writeln!(self.dest, "{indent}<pre><code class=\"language-{lang}\">{}</code></pre>", HtmlText(code))
    }

    fn gen_paragraph(&mut self, text: &Inline, indent: usize) -> Result<(), io::Error> {
        let indent = " ".repeat(indent);
        writeln!(self.dest, "{indent}<p>{text}</p>")
    }
}