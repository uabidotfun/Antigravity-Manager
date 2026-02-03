use reqwest::Proxy;

fn main() {
    let url = "20.74.81.22:16787";
    match Proxy::all(url) {
        Ok(_) => println!("Proxy::all('{}') success", url),
        Err(e) => println!("Proxy::all('{}') failed: {}", url, e),
    }

    let url_http = "http://20.74.81.22:16787";
    match Proxy::all(url_http) {
        Ok(_) => println!("Proxy::all('{}') success", url_http),
        Err(e) => println!("Proxy::all('{}') failed: {}", url_http, e),
    }
}
