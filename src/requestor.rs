use reqwest::{self, header::{USER_AGENT, self, SERVER}, Response, Client, Url};
use base64;
use std::time::Instant;

mod config;
mod parsers;

use super::types;

// makes a request with a random user-agent
async fn make_req(client: Client, url: &str) -> Response {
    let result = client.get(url)
        .header(USER_AGENT, config::get_random_user_agent())
        .send()
        .await // no '?' because we'd have to use Result as return type
        .unwrap();
    result
}

// fn create_robots_txt_url(url: Url) -> String {
//     const ROBOTS_TXT: &'static str = "robots.txt";
//     let schema = url.scheme();
//     let host = url.host_str().unwrap();
//     let port = url.port_or_known_default().unwrap();
//     let url: String = format!("{}://{}:{}/{}", schema, host, port, ROBOTS_TXT);
//     url
// }

pub async fn get_site(
    url: Url,
) -> (
    types::DocumentInfo,
    types::ReqInfo,
    types::FrameworkInfo
) {
    let start = Instant::now();

    let client = reqwest::Client::new();
    let result: Response = make_req(client.clone(), url.as_str()).await;

    // let robots_url = create_robots_txt_url(url).to_owned();
    // let result_robots = make_req(client.clone(), &robots_url).await;
    // println!("robots.txt {}", &result_robots.text().await.unwrap());

    let status = &result.status().clone();
    let status_code = status.to_string();
    let status_reason = status.canonical_reason().unwrap_or("").to_string();
    let status: types::ResStatus = types::ResStatus { 
        status_code: status_code, 
        status_reason: status_reason,
    };

    // Headers before .text()
    let boxed_result: Box<header::HeaderMap> = Box::new(result.headers().clone());
    let leaked_res = Box::leak(boxed_result);
    let headers_clone = leaked_res;
    
    let mut server: String = "".to_string();
    let mut headers: Vec<types::ResHeader> = Vec::new();
    for (key, value) in headers_clone.iter() {
        let value_string = value.to_str().unwrap_or(&"").to_string(); // unwrap_or because it fails with UTF-8 Symbols lol
        
        if key == SERVER {
            server = value_string.clone();
        }
        
        let header_singular: types::ResHeader = types::ResHeader {
            name: key.to_string(),
            value: value_string,
        };
        headers.push(header_singular);
    }

    // .text() destroys the variable, like kinda
    let source_code = result.text().await.unwrap();
    let source_code_b64 = base64::encode(&source_code);
    let parse_result: parsers::DocumentSubsetInfo = parsers::parse_html(&source_code);

    let duration: String = start.elapsed().as_millis().to_string() + " ms";

    let document_info: types::DocumentInfo = types::DocumentInfo {
        source_code: source_code_b64,
        page_title: parse_result.title,
        css_urls: parse_result.css_urls,
        js_urls: parse_result.js_urls,
        img_urls: parse_result.img_urls,
        link_urls: parse_result.link_urls,
    };

    let req_info: types::ReqInfo = types::ReqInfo{
        headers,
        is_alive: true,
        status,
        timing: types::ResTiming { response_time: duration },
    };

    let framework_info: types::FrameworkInfo = types::FrameworkInfo {
        name: parse_result.generator_info.join(", "),
        version: "tbd".to_string(),
        server: server,
        detected_vulnerabilities: vec![]
    };

    (document_info, req_info, framework_info)
}
