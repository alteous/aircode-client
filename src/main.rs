#[macro_use]
extern crate json;
extern crate notify;
extern crate promptly;
extern crate reqwest;
extern crate rustyline;
extern crate scraper;

use notify::Watcher;
use promptly::Prompter;
use std::{fs, io, path};
use std::sync::mpsc::channel;
use std::time::Duration;

const REMOTE_URL: &'static str = "http://192.168.0.10:42000";

struct ProjectSelector(Vec<String>);

impl rustyline::completion::Completer for ProjectSelector {
    fn complete(&self, line: &str, _: usize) -> rustyline::Result<(usize, Vec<String>)> {
        let completions = self.0
            .iter()
            .filter(|name| name.starts_with(&line))
            .map(|name| name.to_string())
            .collect();
        Ok((0, completions))
    }
}

fn read_to_string<P>(path: P) -> io::Result<String>
    where P: AsRef<path::Path>
{
    use io::Read;
    let file = fs::File::open(path)?;
    let size = file.metadata().map(|x| x.len()).unwrap_or(0);
    let mut contents = String::with_capacity(size as usize);
    let mut reader = io::BufReader::new(file);
    reader.read_to_string(&mut contents)?;
    Ok(contents)
}

fn update(project_name: &str, file_name: &str) {
    let url = format!("{}/projects/{}/__update", REMOTE_URL, project_name);
    println!("POST {}", url);
    let path = format!("project/{}.lua", file_name);
    let code = read_to_string(&path).unwrap();
    let body = json::stringify(object! { "contents" => code, "file" => file_name });
    let client = reqwest::Client::new();
    let request = client.post(&url).body(body).build().unwrap();
    println!("{:#?}", request);
    let response = client.execute(request).unwrap();
    println!("{:#?}", response);
}

fn contents() -> Vec<String> {
    println!("GET {}", REMOTE_URL);
    let mut response = reqwest::get(REMOTE_URL).unwrap();
    let text = response.text().unwrap();
    println!("{:#?}", response);

    let document = scraper::Html::parse_document(&text);
    let selector = scraper::Selector::parse("div.project-title").unwrap();
    document.select(&selector).map(|node| node.inner_html()).collect()
}

fn load(project_name: &str) -> Vec<String> {
    let url = format!("{}/projects/{}/Main", REMOTE_URL, project_name);
    println!("GET {}", &url);
    let mut response = reqwest::get(&url).unwrap();
    let text = response.text().unwrap();
    println!("{:#?}", response);

    let mut whitelist = Vec::new();
    let document = scraper::Html::parse_document(&text);
    let selector = scraper::Selector::parse("li:not(.backarrow)").unwrap();
    for node in document.select(&selector) {
        let file_name = node.inner_html();
        read(project_name, &file_name);
        whitelist.push(format!("{}.lua", file_name));
    }

    whitelist
}

fn read(project_name: &str, file_name: &str) {
    use io::Write;

    let url = format!("{}/projects/{}/{}", REMOTE_URL, project_name, file_name);
    println!("GET {}", &url);
    let mut response = reqwest::get(&url).unwrap();
    let text = response.text().unwrap();
    println!("{:#?}", response);

    let document = scraper::Html::parse_document(&text);
    let selector = scraper::Selector::parse("div#editor").unwrap();
    let node = document.select(&selector).next().unwrap();
    let code = node.inner_html();
    let path = format!("project/{}.lua", file_name);
    let mut file = fs::File::create(&path).unwrap();
    file.write_all(code.as_bytes()).unwrap();
}

fn clear() {
    fs::remove_dir_all("project").unwrap();
    fs::create_dir("project").unwrap();
}

fn _restart(project_name: &str) {
    let url = format!("{}/projects/{}/__restart", REMOTE_URL, project_name);
    println!("GET {}", &url);
    let response = reqwest::get(&url).unwrap();
    println!("{:#?}", response);
}

fn main_loop(project_name: &str, whitelist: &[String]) {
    let (tx, rx) = channel();
    let mut watcher = notify::watcher(tx, Duration::from_secs(1)).unwrap();
    watcher.watch("project", notify::RecursiveMode::Recursive).unwrap();
    loop {
        match rx.recv() {
            Ok(notify::DebouncedEvent::Write(path)) => {
                let file_name = path.file_name().unwrap().to_str().unwrap();
                for candidate in whitelist.iter() {
                    if candidate.as_str() == file_name {
                        println!("Update {}", file_name);
                        let file_name_without_extension = file_name
                            .split(".lua")
                            .next()
                            .unwrap();
                        update(project_name, file_name_without_extension);
                        break;
                    }
                }
            },
            Err(error) => println!("error: {:#?}", error),
            _ => {},
        }
    }
}

fn main() {
    let project_names = contents();
    let project_selector = ProjectSelector(project_names);
    let mut prompter = Prompter::with_completer(project_selector);
    let project_name = prompter.prompt_once("Open project");
    clear();
    let whitelist = load(&project_name);
    main_loop(&project_name, whitelist.as_slice());
}

