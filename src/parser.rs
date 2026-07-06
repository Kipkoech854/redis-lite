use std::{
    io::{BufReader, prelude::*},
    net::TcpStream,
};

use crate::server::{self, HttpResponse};

pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
}

pub enum ParseResult {
    Success(HttpRequest),
    Failure(HttpResponse),
}

pub fn parse_request(buf_reader: &mut BufReader<&TcpStream>) -> ParseResult {
    // If parsing fails at the IO level, we return a 500 error
    let request_line = match buf_reader.lines().next() {
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

    ParseResult::Success(HttpRequest {
        method: parts[0].to_string(),
        path: parts[1].to_string(),
        version: parts[2].to_string(),
    })
}
