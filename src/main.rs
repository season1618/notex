pub mod data;
pub mod multiset;
pub mod parser;
pub mod template;
pub mod codegen;

use std::env;
use std::fs::{self, File};
use std::path::Path;

use crate::parser::parse;
use crate::template::read_template;
use crate::codegen::gen_html;

fn main(){
    let args: Vec<String> = env::args().collect();
    let temp_path = &args[1];
    let src_path = &args[2];
    let dest_path = &format!("{}.html", src_path.trim_end_matches(".md"));
    let dest_path = if args.len() <= 3 { &dest_path } else { &args[3] };

    let Ok(doc) = fs::read_to_string(src_path) else {
        println!("could not open the source file.");
        return;
    };

    let (title, toc, content) = match parse(&doc) {
        Ok(res) => res,
        Err(err) => { println!("Syntax Error: {err}"); return; },
    };
    let file = Path::new(dest_path).file_stem().unwrap().to_str().unwrap();

    let Ok(temp) = read_template(temp_path) else {
        println!("could not open or read the template file.");
        return;
    };

    let Ok(mut dest) = File::create(dest_path) else {
        println!("could not open or create the destination file.");
        return;
    };
    
    let Ok(_) = gen_html(&mut dest, file, &title, &toc, &content, &temp) else {
        println!("could not write to the destination file.");
        return;
    };
}