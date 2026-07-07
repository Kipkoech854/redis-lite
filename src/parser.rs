use std::{
    collections::HashMap,
    io::{BufReader, prelude::*},
    net::TcpStream,
};

use crate::server::{self, HttpResponse};

pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

pub enum ParseResult {
    Success(HttpRequest),
    Failure(HttpResponse),
}

pub fn parse_request(buf_reader: &mut BufReader<&TcpStream>) -> ParseResult {
    // If parsing fails at the IO level, we return a 500 error
    let mut lines = buf_reader.lines();
    let request_line = match lines.next() {
        Some(Ok(line)) => line,
        _ => {
            return ParseResult::Failure(HttpResponse::new(
                "500",
                String::from("Error parsing request: Request line not found"),
            ));
        }
    };

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    //if request line does not have all parts we return a 400 error
    if parts.len() < 3 {
        return ParseResult::Failure(HttpResponse::new(
            "400",
            String::from("Error parsing request: Invalid request line"),
        ));
    }
    let method = parts[0].to_string();

    //parse headers
    let mut headers = HashMap::new();
    let mut content_length: usize = 0;

    //Continue looping through lines till we hit \n
    for line_result in lines {
        let line = match line_result {
            Ok(line) => line,
            Err(_) => {
                return ParseResult::Failure(HttpResponse::new(
                    "400",
                    "Error reading headers".to_string(),
                ));
            }
        };
        //an empty line marks end of headers
        if line.is_empty() {
            break;
        }
        //split header into Key:Value
        if let Some((key, val)) = line.split_once(":") {
            let key = key.trim().to_lowercase();
            let val = val.trim().to_string();
            //keep track of Content-length if its POST
            if key == "content-length" {
                content_length = val.parse().unwrap_or(0);
            }
            headers.insert(key, val);
        }
    }
    //Parse body if only content-length is >0
    // 3. Parse the Body (Strictly bound by HTTP method rules)
    let body = if method == "POST" || method == "PUT" {
        if content_length > 0 {
            let mut body_bytes = vec![0; content_length];
            if buf_reader.read_exact(&mut body_bytes).is_err() {
                return ParseResult::Failure(HttpResponse::new(
                    "400",
                    "Error reading request body".to_string(),
                ));
            }
            Some(String::from_utf8_lossy(&body_bytes).into_owned())
        } else {
            // A POST/PUT with no body content
            None
        }
    } else {
        // GET, DELETE, OPTIONS, etc. drop straight to None instantly
        None
    };

    ParseResult::Success(HttpRequest {
        method: method,
        path: parts[1].to_string(),
        version: parts[2].to_string(),
        headers,
        body,
    })
}
fn is_cacheable(key: &str) -> bool {}
