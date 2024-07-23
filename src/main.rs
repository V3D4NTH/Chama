use std::env;
use std::fs::{File, OpenOptions};use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::thread;

fn handle_connection(mut stream: std::net::TcpStream, directory: &str) {
    // Buffer to store the request
    let mut buffer = [0; 1024];
    let bytes_read = stream.read(&mut buffer).unwrap();

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let mut headers = request.lines();

    let request_line = headers.next().unwrap();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap();
    let url_path = parts.next().unwrap();

    // Determine if gzip encoding is supported
    let mut accept_gzip = false;
    for header in headers.clone() {
        if header.to_lowercase().starts_with("accept-encoding:") {
            let encodings: Vec<&str> = header.split(':').nth(1).unwrap().split(',').map(|s| s.trim()).collect();
            if encodings.contains(&"gzip") {
                accept_gzip = true;
                break;
            }
        }
    }

    if url_path.starts_with("/files/") {
        let filename = &url_path[7..]; // Extract the filename after "/files/"
        let filepath = format!("{}/{}", directory, filename);

        if method == "GET" {
            if Path::new(&filepath).exists() {
                let mut file = File::open(&filepath).unwrap();
                let mut contents = Vec::new();
                file.read_to_end(&mut contents).unwrap();
                let content_length = contents.len();

                let mut response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n",
                    content_length
                );

                if accept_gzip {
                    response.push_str("Content-Encoding: gzip\r\n");
                }
                response.push_str("\r\n");
                stream.write_all(response.as_bytes()).unwrap();
                stream.write_all(&contents).unwrap();
            } else {
                let response = "HTTP/1.1 404 Not Found\r\n\r\n".to_string();
                stream.write_all(response.as_bytes()).unwrap();
            }
        } else if method == "POST" {
            // Read headers to find Content-Length
            let mut content_length = 0;
            for header in headers.clone() {
                if header.to_lowercase().starts_with("content-length:") {
                    content_length = header.split(':').nth(1).unwrap().trim().parse().unwrap();
                    break;
                }
            }

            // Calculate how much of the body is already read
            let body_start = request.find("\r\n\r\n").unwrap() + 4;
            let already_read_body = &buffer[body_start..bytes_read];
            let mut body = Vec::from(already_read_body);

            // Read the rest of the request body if not fully read
            if body.len() < content_length {
                let mut remaining_body = vec![0; content_length - body.len()];
                stream.read_exact(&mut remaining_body).unwrap();
                body.extend_from_slice(&remaining_body);
            }

            // Write the request body to the file
            let mut file = OpenOptions::new().write(true).create(true).open(&filepath).unwrap();
            file.write_all(&body).unwrap();

            // Respond with 201 Created
            let response = "HTTP/1.1 201 Created\r\n\r\n".to_string();
        
            stream.write_all(response.as_bytes()).unwrap();
        } else {
            let response = "HTTP/1.1 405 Not Allowed\r\n\r\n".to_string();
            stream.write_all(response.as_bytes()).unwrap();
        }
    } else if url_path == "/user-agent" {
        // Initialize the User-Agent header value
        let mut user_agent = "";

        // Loop through the headers to find the User-Agent header
        for header in headers {
            if header.to_lowercase().starts_with("user-agent:") {
                user_agent = header.split(':').nth(1).unwrap().trim();
                break;
            }
        }

        // Construct the response
        let content_length = user_agent.len();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
            content_length, user_agent
        );

        // Write the response to the stream
        stream.write_all(response.as_bytes()).unwrap();
    } else if url_path.starts_with("/echo/") {
        let response_str = &url_path[6..]; // Extract the string after "/echo/"
        let content_length = response_str.len();
        let mut response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n",
            content_length
        );
        if accept_gzip {
            response.push_str("Content-Encoding: gzip\r\n");
        }

        response.push_str("\r\n");
        stream.write_all(response.as_bytes()).unwrap();
        stream.write_all(response_str.as_bytes()).unwrap();
    } else if url_path == "/" {
        let response = "HTTP/1.1 200 OK\r\n\r\n".to_string();
        stream.write_all(response.as_bytes()).unwrap();
    } else {
        // Respond with 404 Not Found for other paths
        let response = "HTTP/1.1 404 Not Found\r\n\r\n".to_string();
        stream.write_all(response.as_bytes()).unwrap();
    }
}

fn main() {
    // Parse the --directory flag from command-line arguments
    let args: Vec<String> = env::args().collect();
    let directory = match args.iter().position(|arg| arg == "--directory") {
        Some(index) => &args[index + 1],
        None => {
            eprintln!("Warning: --directory flag not provided. Using current directory as default.");
            "."
        }
    };

    // Bind the listener to the address
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                let directory = directory.to_string();
                // Spawn a new thread to handle the connection
                thread::spawn(move || {
                    handle_connection(stream, &directory);
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}