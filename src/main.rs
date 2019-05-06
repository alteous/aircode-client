use json::object;
use marksman_escape::Unescape;
use notify::Watcher;
use promptly::Prompter;
use std::{fs, io, path};
use std::sync::mpsc::channel;
use std::time::Duration;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "aircode-client", about = "Unofficial aircode client")]
struct Options {
    /// Aircode URL.
    #[structopt(short = "u", long = "url")]
    url: String,
}

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

fn update(options: &Options, project_name: &str, basename: &str) {
    let url = format!("{}/projects/{}/__update", options.url, project_name);
    println!("POST {}", url);
    let path = format!("project/{}.lua", basename);
    let code = read_to_string(&path).unwrap();
    let body = json::stringify(object! { "contents" => code, "file" => basename });
    let client = reqwest::Client::new();
    let request = client.post(&url).body(body).build().unwrap();
    println!("{:#?}", request);
    let response = client.execute(request).unwrap();
    println!("{:#?}", response);
}

fn contents(options: &Options) -> Vec<String> {
    println!("GET {}", options.url);
    let mut response = reqwest::get(&options.url).unwrap();
    let text = response.text().unwrap();
    println!("{:#?}", response);

    let document = scraper::Html::parse_document(&text);
    let selector = scraper::Selector::parse("div.project-title").unwrap();
    document.select(&selector).map(|node| node.inner_html()).collect()
}

fn load(options: &Options, project_name: &str) -> Vec<String> {
    let url = format!("{}/projects/{}/Main", options.url, project_name);
    println!("GET {}", &url);
    let mut response = reqwest::get(&url).unwrap();
    let text = response.text().unwrap();
    println!("{:#?}", response);

    let mut whitelist = Vec::new();
    let document = scraper::Html::parse_document(&text);
    let selector = scraper::Selector::parse("li:not(.backarrow)").unwrap();
    for node in document.select(&selector) {
        let file_name = node.inner_html();
        read(options, project_name, &file_name);
        whitelist.push(format!("{}.lua", file_name));
    }

    whitelist
}

fn read(options: &Options, project_name: &str, basename: &str) {
    use io::Write;

    let url = format!("{}/projects/{}/{}", options.url, project_name, basename);
    println!("GET {}", &url);
    let mut response = reqwest::get(&url).unwrap();
    let text = response.text().unwrap();
    println!("{:#?}", response);

    let document = scraper::Html::parse_document(&text);
    let selector = scraper::Selector::parse("div#editor").unwrap();
    let node = document.select(&selector).next().unwrap();
    let code = node.inner_html();
    let unescaped_code = String::from_utf8(Unescape::new(code.bytes()).collect()).unwrap();
    let path = format!("project/{}.lua", basename);
    let mut file = fs::File::create(&path).unwrap();
    file.write_all(unescaped_code.as_bytes()).unwrap();
}

fn clear() {
    fs::remove_dir_all("project").unwrap();
    fs::create_dir("project").unwrap();
    fs::File::create("project/restart").unwrap();
}

fn restart(options: &Options, project_name: &str) {
    let url = format!("{}/projects/{}/__restart", options.url, project_name);
    println!("GET {}", &url);
    let response = reqwest::get(&url).unwrap();
    println!("{:#?}", response);
}

fn restart_helper(options: &Options, project_name: &str, path: &path::Path) {
    if let Some(os_str) = path.file_name() {
        if let Some(name) = os_str.to_str() {
            if name == "restart" {
                restart(options, project_name);
            }
        }
    }
}

fn update_helper(options: &Options, project_name: &str, path: &path::Path, whitelist: &[String]) {
    let file_name = path.file_name().unwrap().to_str().unwrap();
    if file_name == "restart" {
        restart(options, project_name);
    } else {
        for candidate in whitelist.iter() {
            if candidate.as_str() == file_name {
                println!("Update {}", file_name);
                let basename = file_name
                    .split(".lua")
                    .next()
                    .unwrap();
                update(options, project_name, basename);
                break;
            }
        }
    }
}

fn main_loop(options: &Options, project_name: &str, whitelist: &[String]) {
    let (tx, rx) = channel();
    let poll_delay = Duration::from_millis(250); // Poll every quarter of a second.
    let mut watcher = notify::watcher(tx, poll_delay).unwrap();
    watcher.watch("project", notify::RecursiveMode::Recursive).unwrap();
    loop {
        match rx.recv() {
            Ok(notify::DebouncedEvent::Write(path)) => {
                update_helper(options, project_name, &path, whitelist);
            },
            Ok(notify::DebouncedEvent::NoticeRemove(path)) => {
                update_helper(options, project_name, &path, whitelist);
            },
            Ok(notify::DebouncedEvent::Create(path)) => {
                restart_helper(options, project_name, &path);
            },
            Ok(notify::DebouncedEvent::NoticeWrite(path)) => {
                restart_helper(options, project_name, &path);
            },
            Ok(notify::DebouncedEvent::Chmod(path)) => {
                restart_helper(options, project_name, &path);
            },
            Ok(event) => {
                println!("{:#?}", event);
            },
            Err(error) => println!("error: {:#?}", error),
        }
    }
}

fn main() {
    let options = Options::from_args();
    let project_names = contents(&options);
    let project_selector = ProjectSelector(project_names);
    let mut prompter = Prompter::with_completer(project_selector);
    let project_name = prompter.prompt_once("Open project");
    clear();
    let whitelist = load(&options, &project_name);
    main_loop(&options, &project_name, whitelist.as_slice());
}
