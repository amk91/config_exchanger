extern crate xmlparser as xml;
use xml::Token;

use std::fs::*;
use std::io::{Read, BufReader};
use std::io::prelude::*;
use std::error::Error;
use std::env::current_dir;
use std::path::PathBuf;

/*
source=doc_1.xml
destination=doc_2.xml
sort_key=name
value_key=value
tags_to_ignore=param::interOptWithCT, group::directories
*/
fn main() {
    let mut source_filepath = String::new();
    let mut destination_filepath = String::new();
    let mut sort_key = String::new();
    let mut value_key = String::new();
    let mut tags_to_ignore: Vec<String> = Vec::new();

    if let Ok(current_dir) = current_dir() {
        if let Ok(directory_elements_list) = read_dir(current_dir) {
            for element in directory_elements_list {
                if let Ok(filepath) = element {
                    let path = filepath.path();
                    let filename = path
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap();
                    if filename == "config.txt" {
                        parse_config_file(
                            &path,
                            &mut source_filepath,
                            &mut destination_filepath,
                            &mut sort_key,
                            &mut value_key,
                            &mut tags_to_ignore
                        );
                        break;
                    }
                }
            }
        }
    }

    println!("###############################", );
    println!("### CONFIGURATION EXCHANGER ###", );
    println!("###############################", );
    println!("");

    println!("Settings");
    println!("------------------------");
    println!("| Source file: {}", source_filepath);
    println!("| Destination file: {}", destination_filepath);
    println!("| Sort field: {}", sort_key);
    println!("| Update field: {}", value_key);
    println!("| Tags to ignore: ");
    tags_to_ignore.iter().for_each(|x| println!("|\t{}", x));

    println!("");
    println!("");

    println!("Press ENTER to begin");
    if let Err(error) = std::io::stdin().read_line(&mut String::new()) {
        panic!("Unable to read from console: {}", error.description());
    }

    let sort_key = Some(sort_key);
    let value_key = Some(value_key);

    let parse_result = parse(
        &source_filepath,
        &sort_key,
        &value_key,
        &tags_to_ignore
    );

    if let Ok(parse_result) = &parse_result {
        write(
            &destination_filepath,
            &sort_key,
            &value_key,
            parse_result
        );
    }

    println!("Process completed. Press ENTER to exit");
    if let Err(error) = std::io::stdin().read_line(&mut String::new()) {
        panic!("Unable to read from console: {}", error.description());
    }
}

fn parse(
    filepath: &str,
    sort_key: &Option<String>,
    value_key: &Option<String>,
    tags_to_ignore: &Vec<String>
) -> Result<Vec<(String, String)>, xml::Error> {
    let mut tag_path = String::from("");

    // This is a vector of pairs where each element is made by the
    // path to reach a specific element and the value of the field
    // we want to update in the destination file
    let mut list: Vec<(String, String)> = Vec::new();

    let text = load_file(filepath);
    for token in xml::Tokenizer::from(text.as_str()) {
        match token.unwrap() {
            Token::ElementStart(_, tag) => {
                tag_path.push_str("::");
                tag_path.push_str(&tag.to_string());
            }
            Token::Attribute((_, key), value) => {
                if let Some(ref sort_key) = sort_key {
                    if key.to_string() == *sort_key {
                        tag_path.push_str("::");
                        tag_path.push_str(&value.to_string());
                    }
                }

                if let Some(ref value_key) = value_key {
                    if key.to_string() == *value_key {
                        tag_path.push_str("??");
                        tag_path.push_str(&value.to_string());
                    }
                }
            }
            Token::ElementEnd(element) => {
                if element != xml::ElementEnd::Open {
                    let value_index = tag_path.rfind("??");
                    if let Some(value_index) = value_index {
                        let mut save_tag_value = true;
                        for ignored_tag in tags_to_ignore {
                            if tag_path.contains(ignored_tag.as_str()) {
                                save_tag_value = false;
                                break;
                            }
                        }

                        let value: String = tag_path
                            .drain((value_index + 2)..)
                            .collect();
                        tag_path.drain(value_index..);
                        if save_tag_value {
                            list.push((tag_path.clone(), value));
                        }
                    }

                    let tag_index = tag_path.rfind("::");
                    if let Some(tag_index) = tag_index {
                        tag_path.drain(tag_index..);
                    }

                    if element == xml::ElementEnd::Empty &&
                        sort_key.is_some()
                    {
                        let tag_index = tag_path.rfind("::");
                        if let Some(tag_index) = tag_index {
                            tag_path.drain(tag_index..);
                        }
                    }
                }
            }
            _ => {

            }
        }
    }

    Ok(list)
}

fn write(
    filepath: &str,
    sort_key: &Option<String>,
    value_key: &Option<String>,
    update_list: &Vec<(String, String)>,
) {
    let mut tag_path = String::from("");
    let text = load_file(filepath);
    let mut new_file = create_file(filepath);
    for token in xml::Tokenizer::from(text.as_str()) {
        match token.unwrap() {
            Token::Declaration(version, encoding, _) => {
                // <?xml version="1.0" encoding="utf-8"?>
                let mut buffer = String::from("<?xml version=\"");
                buffer.push_str(version.to_str());
                buffer.push_str("\"");

                if let Some(encoding) = encoding {
                    buffer.push_str(" encoding=\"");
                    buffer.push_str(encoding.to_str());
                    buffer.push_str("\"");
                }

                buffer.push_str("?>\r\n");
                write_on_file(&mut new_file, buffer, "Declaration");
            },
            Token::Comment(comment) => {
                // <!-- text -->
                let mut buffer = String::from("<!-- ");
                buffer.push_str(comment.to_str());
                buffer.push_str(" -->");

                write_on_file(&mut new_file, buffer, "Comment");
            },
            Token::Text(text) => {
                let mut buffer = String::from(text.to_str());

                write_on_file(&mut new_file, buffer, "Text");
            },
            Token::Whitespaces(whitespaces) => {
                let mut buffer = String::from(whitespaces.to_str());

                write_on_file(&mut new_file, buffer, "Whitespaces");
            },
            Token::ElementStart(_, tag) => {
                let mut buffer = String::from("<");
                buffer.push_str(tag.to_str());

                write_on_file(&mut new_file, buffer, "ElementStart");

                tag_path.push_str("::");
                tag_path.push_str(tag.to_str());
            },
            Token::Attribute((_, key), value) => {
                let mut buffer = String::from(" ");
                buffer.push_str(key.to_str());
                buffer.push_str("=\"");

                if let Some(ref sort_key) = sort_key {
                    if key.to_string() == *sort_key {
                        tag_path.push_str("::");
                        tag_path.push_str(value.to_str());
                    }
                }

                let mut new_value = String::from(value.to_string());
                if let Some(ref value_key) = value_key {
                    if key.to_string() == *value_key {
                        for (tag_to_update, value) in update_list {
                            if tag_path.contains(tag_to_update) {
                                new_value = value.clone();
                                break;
                            }
                        }
                    }
                }

                buffer.push_str(&new_value);
                buffer.push_str("\"");

                write_on_file(&mut new_file, buffer, "Attribute");
            },
            Token::ElementEnd(element) => {
                let mut buffer = String::new();

                if element == xml::ElementEnd::Open {
                    buffer.push_str(">");

                    write_on_file(&mut new_file, buffer, "ElementEnd(Open)");
                } else {
                    match element {
                        xml::ElementEnd::Close(_, name) => {
                            buffer.push_str("</");
                            buffer.push_str(name.to_str());
                            buffer.push_str(">");

                            write_on_file(&mut new_file, buffer, "ElementEnd(Close)");
                        },
                        xml::ElementEnd::Empty => {
                            buffer.push_str(" />");

                            write_on_file(&mut new_file, buffer, "ElementEnd(Empty)");
                        }
                        _ => {

                        }
                    }

                    let value_index = tag_path.rfind("??");
                    if let Some(value_index) = value_index {
                        // let value: String = tag_path
                        //     .drain((value_index + 2)..)
                        //     .collect();
                        tag_path.drain(value_index..);
                    }

                    let tag_index = tag_path.rfind("::");
                    if let Some(tag_index) = tag_index {
                        tag_path.drain(tag_index..);
                    }

                    if element == xml::ElementEnd::Empty &&
                        sort_key.is_some()
                    {
                        let tag_index = tag_path.rfind("::");
                        if let Some(tag_index) = tag_index {
                            tag_path.drain(tag_index..);
                        }
                    }
                }
            },
            _ => {

            },
        }
    }
}

fn load_file(path: &str) -> String {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) => {
            panic!("Error on loading file {}: {}",
                path,
                error.description()
            );
        }
    };

    let mut text = String::new();
    let _ = match file.read_to_string(&mut text) {
        Ok(_) => { },
        Err(error) => {
            panic!("Error on reading file {}: {}",
                path,
                error.description()
            );
        },
    };

    text
}

fn create_file(path: &str) -> File {
    match rename(path, String::from(path) + ".OLD") {
        Err(error) => {
            panic!(
                "Error on renaming {}: {}",
                path,
                error.description()
            );
        },
        _ => {

        },
    }

    let file = match File::create(path) {
        Ok(file) => file,
        Err(error) => {
            panic!("Error on creating file: {}",
                error.description());
        }
    };

    file
}

fn write_on_file(file: &mut File, buffer: String, section: &str) {
    match file.write_all(buffer.as_bytes()) {
        Err(error) => {
            panic!(
                "Unable to write data in the 
                    new file on {}: {}",
                section,
                error.description()
            );
        },
        _ => {

        }
    }
}

fn parse_config_file(
    filepath: &PathBuf,
    source_filepath: &mut String,
    destination_filepath: &mut String,
    sort_key: &mut String,
    value_key: &mut String,
    tags_to_ignore: &mut Vec<String>,
    ) {
    let file = match File::open(filepath) {
        Ok(file) => file,
        Err(error) => {
            panic!(
                "Error opening config file: {}",
                error.description()
            );
        },
    };

    let file = BufReader::new(&file);
    let mut lines = file.lines();

    // Search for source filepath
    let mut first_line_found = false;
    while let Some(Ok(line)) = lines.next() {
        if line.starts_with("source=") {
            first_line_found = true;
            if let Some(source) = line.split("=").nth(1) {
                *source_filepath = source.to_string();
            }

            break;
        }
    }

    if !first_line_found {
        panic!("Wrong cfg format on source");
    }

    // Search for destination filepath
    if let Some(Ok(line)) = lines.next() {
        if line.starts_with("destination=") {
            if let Some(destination) = line.split("=").nth(1) {
                *destination_filepath = destination.to_string();
            }
        } else {
            panic!("Wrong cfg format on destination");
        }
    }

    // Search for sort key
    if let Some(Ok(line)) = lines.next() {
        if line.starts_with("sort_key=") {
            if let Some(sort) = line.split("=").nth(1) {
                *sort_key = sort.to_string();
            }
        } else {
            panic!("Wrong cfg format on sort_key");
        }
    }

    // Search for value key
    if let Some(Ok(line)) = lines.next() {
        if line.starts_with("value_key=") {
            if let Some(value) = line.split("=").nth(1) {
                *value_key = value.to_string();
            }
        } else {
            panic!("Wrong cfg format on value_key");
        }
    }

    // Search for tags to ignore
    if let Some(Ok(line)) = lines.next() {
        if line.starts_with("tags_to_ignore=") {
            if let Some(tags) = line.split("=").nth(1) {
                tags.split(",").for_each(|x| {
                    tags_to_ignore.push(x.trim().to_owned());
                })
            }
        } else {
            panic!("Wrong cfg format on tags_to_ignore");
        }
    }
}
