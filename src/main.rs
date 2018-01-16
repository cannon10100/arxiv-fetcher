extern crate futures;
extern crate tokio_core;
extern crate hyper;
extern crate atom_syndication;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

use std::thread;
use std::io::{self, Write};
use std::sync::Mutex;
use std::fs::File;
use std::io::prelude::*;
use std::collections::{HashSet, HashMap};
use futures::{Future, Stream};
use hyper::Client;
use tokio_core::reactor::Core;
use atom_syndication::Feed;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DocInfo {
    id: String,
    title: String,
    summary: String
}

lazy_static! {
    static ref CATEGORIES: [&'static str; 6] = ["cs.ai", "cs.lg", "cs.ma", "cs.ne", "cs.ro", "cs.sy"];
    static ref FETCHED_DOCUMENTS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
    static ref DOCUMENTS_READ: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
    static ref DOCUMENT_INFO: Mutex<HashMap<String, DocInfo>> = Mutex::new(HashMap::new());
}

static ARXIV_REQUEST_FIRST: &str = "http://export.arxiv.org/api/query?search_query=";
static ARXIV_REQUEST_LAST: &str = "&start=0&max_results=100&sortBy=submittedDate&sortOrder=descending";
static ID_FILENAME: &str = "doc_ids.json";
static DOCS_READ_FILENAME: &str = "docs_read.json";
static DOCS_INFO_FILENAME: &str = "docs_info.json";

// Save retrieved document ids to a file locally
fn save_ids(filename: &str, read_filename: &str, info_filename: &str) {
    let serialized = serde_json::to_string(&*FETCHED_DOCUMENTS.lock().unwrap()).unwrap();
    println!("D: Saving {} IDs: {}", FETCHED_DOCUMENTS.lock().unwrap().len(), serialized);

    let mut file = File::create(filename).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();

    // Save document info
    let info_serialized = serde_json::to_string(&*DOCUMENT_INFO.lock().unwrap()).unwrap();
    println!("D: Saving {} IDs' info", DOCUMENT_INFO.lock().unwrap().len());

    let mut file = File::create(info_filename).unwrap();
    file.write_all(info_serialized.as_bytes()).unwrap();

    // Save read documents
    let read_serialized = serde_json::to_string(&*DOCUMENTS_READ.lock().unwrap()).unwrap();
    println!("D: Saving {} Read IDs: {}", DOCUMENTS_READ.lock().unwrap().len(), read_serialized);

    let mut file = File::create(read_filename).unwrap();
    file.write_all(read_serialized.as_bytes()).unwrap();
}

// Load saved ids from file
fn load_ids(filename: &str, read_filename: &str, info_filename: &str) {
    let file = File::open(filename);
    if let Ok(mut file) = file {
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        let parsed_ids: Vec<String> = serde_json::from_str(&contents).unwrap();
        println!("Loaded IDs {:?}", parsed_ids);

        let mut mut_guard = FETCHED_DOCUMENTS.lock().unwrap();
        for id in parsed_ids {
            mut_guard.insert(id);
        }
    }

    let info_file = File::open(info_filename);
    if let Ok(mut info_file) = info_file {
        let mut contents = String::new();
        info_file.read_to_string(&mut contents).unwrap();

        let parsed_info: HashMap<String, DocInfo> = serde_json::from_str(&contents).unwrap();
        println!("Loaded ID info {:?}", parsed_info);

        let mut info_mut_guard = DOCUMENT_INFO.lock().unwrap();
        for (id, info) in &parsed_info {
            info_mut_guard.insert(String::from(&*id.clone()), (*info).clone());
        }
    }

    let read_file = File::open(read_filename);
    if let Ok(mut read_file) = read_file {
        let mut contents = String::new();
        read_file.read_to_string(&mut contents).unwrap();

        let parsed_read_ids: Vec<String> = serde_json::from_str(&contents).unwrap();
        println!("Loaded read IDs {:?}", parsed_read_ids);

        let mut read_mut_guard = DOCUMENTS_READ.lock().unwrap();
        for id in parsed_read_ids {
            read_mut_guard.insert(id);
        }
    }
}

fn make_request() -> hyper::Result<&'static str> {
    let mut core = Core::new()?;
    let mut documents = vec!();
    let client = Client::new(&core.handle());
    for topic in CATEGORIES.iter() {
        println!("\nFetching new results from {}", topic);
        let topic_url = format!("{}{}{}", ARXIV_REQUEST_FIRST, topic, ARXIV_REQUEST_LAST);
        let uri = topic_url.parse()?;
        let work = client.get(uri).and_then(|res| {
            println!("Response: {}", res.status());

            res.body().concat2().and_then(|body| {
                let body_str = std::str::from_utf8(&body)?;
                documents.push(String::from(body_str));
                Ok(())
            })
        });
        core.run(work)?;
        thread::sleep_ms(3000);
    }

    for request in documents.iter() {
        let feed = request.parse::<Feed>().unwrap();
        for entry in feed.entries() {
            println!("Got document with title: {}", entry.title());
            FETCHED_DOCUMENTS.lock().unwrap().insert(String::from(entry.id()));
        }
    }

    println!("Fetched documents:");
    for doc in FETCHED_DOCUMENTS.lock().unwrap().iter() {
        println!("\t{}", doc);
    }

    save_ids(ID_FILENAME, DOCS_READ_FILENAME, DOCS_INFO_FILENAME);

    return Ok("Done");
}

fn main() {
    load_ids(ID_FILENAME, DOCS_READ_FILENAME, DOCS_INFO_FILENAME);
    println!("{:?}", make_request());
}