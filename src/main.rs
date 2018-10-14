#[macro_use]
extern crate json;
extern crate reqwest;

use std::env;

const REMOTE_URL: &'static str = "http://192.168.0.10:42000";

const CODE: &'static str = r#"function setup()
    print "Hello world!"
    i = 0
end

function draw()
    i = i + 40.0 * DeltaTime
    local c = math.min(i, 60)
    background(c, c, c)
    spriteMode(CENTER)
    sprite("Blocks:Brick Grey", 0.5 * WIDTH, 0.5 * HEIGHT)
end"#;

fn update(project_name: &str, file_name: &str) {
    let url = format!("{}/projects/{}/__update", REMOTE_URL, project_name);
    println!("POST {}", url);
    let code = CODE;
    let body = json::stringify(object! { "contents" => code, "file" => file_name });
    let client = reqwest::Client::new();
    let request = client.post(&url).body(body).build().unwrap();
    println!("{:#?}", request);
    let response = client.execute(request).unwrap();
    println!("{:#?}", response);
}

fn contents() {
    println!("GET {}", REMOTE_URL);
    let mut response = reqwest::get(REMOTE_URL).unwrap();
    let text = response.text().unwrap();
    println!("{:#?}", response);
    println!("{:#?}", text);
}

fn open(project_name: &str, file_name: Option<&str>) {
    let url = format!("{}/projects/{}/{}", REMOTE_URL, project_name, file_name.unwrap_or("Main"));
    println!("GET {}", &url);
    let mut response = reqwest::get(&url).unwrap();
    let text = response.text().unwrap();
    println!("{:#?}", response);
    println!("{:#?}", text);
}

fn restart(project_name: &str) {
    let url = format!("{}/projects/{}/__restart", REMOTE_URL, project_name);
    println!("GET {}", &url);
    let response = reqwest::get(&url).unwrap();
    println!("{:#?}", response);
}

fn main() {
    let arg = env::args().nth(1);
    match arg.as_ref().map(String::as_str) {
        Some("restart") => restart("Test"),
        Some("update") => update("Test", "Main"),
        Some("open") => open("Test", None),
        Some("contents") => contents(),
        _ => println!("usage: <restart/update/open/contents> [path]"),
    }
}

