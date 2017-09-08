use std::path::Path;
use std::collections::HashMap;
use std::fs::{File, metadata, read_dir};
use std::io::{Read, Result};
extern crate pulldown_cmark;
use self::pulldown_cmark::{Parser, Options, OPTION_ENABLE_TABLES, Event, Tag};

#[derive(Default, Debug)]
pub struct Table {
    pub header: String,
    pub subheader: String,
    pub columns: HashMap<String, Vec<String>>,
}

fn read_str(string: &str) -> Result<Vec<Table>> {
    let mut options = Options::empty();
    options.insert(OPTION_ENABLE_TABLES);
    let parser = Parser::new_ext(string, options);
    let mut current_headers = vec![String::new(), String::new()];
    let mut last_string = String::new();
    let mut in_table_head = false;
    let mut tables = Vec::new();
    let mut column_names = Vec::new();
    let mut current_column_index = 0;
    for event in parser {
        match event {
            Event::End(Tag::Header(level)) => {
                current_headers[(level - 1) as usize] = last_string.clone();
                last_string = String::new()
            }
            Event::Start(Tag::TableHead) => {
                tables.push(Table {
                    header: current_headers[0].clone(),
                    subheader: current_headers[1].clone(),
                    columns: Default::default(),
                });
                in_table_head = true
            }
            Event::End(Tag::TableHead) => in_table_head = false,
            Event::End(Tag::TableRow) => current_column_index = 0,
            Event::End(Tag::TableCell) => {
                if in_table_head {
                    column_names.push(last_string.clone());
                    tables
                        .last_mut()
                        .expect("Table head should have started")
                        .columns
                        .insert(last_string.clone(), Vec::new());
                } else {
                    tables
                        .last_mut()
                        .expect("Table body should have started")
                        .columns
                        .get_mut(&column_names[current_column_index])
                        .expect("Column should exist")
                        .push(last_string.clone());
                    current_column_index += 1;
                }
                last_string = String::new();
            }
            Event::Text(string) => last_string = String::from(string.trim()),
            _ => {}
        }
    }
    Ok(tables)
}

fn read_file<P: AsRef<Path>>(path: &P) -> Result<Vec<Table>> {
    let mut string = String::new();
    File::open(path)?.read_to_string(&mut string)?;
    read_str(&string)
}

pub fn read<P: AsRef<Path>>(path: &P) -> Result<Vec<Table>> {
    if metadata(path)?.is_file() {
        read_file(path)
    } else {
        Ok(
            read_dir(path)?
                .flat_map(|file_path| {
                    read_file(&file_path?.path()).map(IntoIterator::into_iter)
                })
                .flat_map(|i| i)
                .collect(),
        )
    }
}
