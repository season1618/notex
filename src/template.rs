use std::io::{self, BufRead, BufReader};
use std::fs::File;
use regex::Regex;

use crate::data::Elem;
use Elem::*;

pub fn read_template(path: &str) -> Result<Vec<Elem>, io::Error> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut template: Vec<Elem> = Vec::new();
    let pattern = Regex::new("\\{[a-z]+\\}").unwrap();

    while reader.read_line(&mut line)? > 0 {
        let mut text_iter = pattern.split(&line);
        let mut attr_iter = pattern.find_iter(&line);
        loop {
            if let Some(text) = text_iter.next() {
                template.push(Str(text.to_string()));
            } else {
                break;
            }
            if let Some(attr) = attr_iter.next() {
                template.push(match attr.as_str() {
                    "{title}" => Title,
                    "{year}" => Year,
                    "{month}" => Month,
                    "{day}" => Day,
                    "{hour}" => Hour,
                    "{minute}" => Minute,
                    "{second}" => Second,
                    "{toc}" => Toc(attr.start()),
                    "{content}" => Content(attr.start()),
                    _ => { println!("unknown attribute"); panic!(); },
                });
            }
        }
        line.clear();
    }

    Ok(template)
}