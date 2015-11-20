extern crate iron;
#[macro_use]
extern crate router;
extern crate nlp;
extern crate serde;
extern crate serde_json;
extern crate threadpool;

use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;
use std::sync::mpsc::channel;
use nlp::distance::levenshtein;
use nlp::distance::jaro;
use nlp::phonetics::metaphone::metaphone;
use iron::prelude::*;
use iron::status;
use router::{Router};
use std::io::{self, BufReader};
use std::io::prelude::*;
use std::fs::File;

use std::path::Path;

fn filename_to_string<'a>(s: &'a str) -> String {
    let path = Path::new(s);
    let mut file = File::open(&path).unwrap();
    let mut s = String::new();
    file.read_to_string(&mut s);
    s
}

fn words_by_line<'a>(s: &'a str) -> Vec<&'a str> {
    s.lines().collect()
}
fn main() {
    let path = Path::new("words2.txt");
    let mut s = String::new();
    let mut file = File::open(&path).unwrap();
    file.read_to_string(&mut s).unwrap();
    let wbyl: Vec<String> = s.lines().map(|s| s.to_owned()).collect();
    let v: Vec<(String, String)> = s.clone().lines().map(|s| s.to_owned()).map(|s| (s.to_owned(), metaphone(&s))).collect();
    let meta_dict_arc = Arc::new(v);
    let v_arc = Arc::new(wbyl);
    let mut router = Router::new();

    fn handler<'a>(req: &mut Request,  v: &'a Vec<String>, meta_dict: &'a Vec<(String, String)>) -> IronResult<Response> {
        let query = req.extensions.get::<Router>().unwrap().find("query").unwrap_or("/").to_owned();
        let mut metaphone_result: Vec<&String> = Vec::new();
        let v_arc = Arc::new(v.clone());
        let pool = ThreadPool::new(2);

        let (tx, rx) = channel();

        let arc_query = Arc::new(query.clone());
        let tx_lev = tx.clone();

        let metaphone_input = metaphone(&query);

        for i in meta_dict {
            if i.1 == metaphone_input {
                metaphone_result.push(&i.0);
            }
        }
        let query_lev = query.clone();

        let v_lev = v_arc.clone();
        pool.execute(move|| {

            for i in v_lev.iter() {
                levenshtein(&query_lev, &i);
            }
            tx_lev.send(vec![""]).unwrap();
        });

        let tx_jaro = tx.clone();

        let query_jaro = query.clone();

        let v_jaro = v_arc.clone();
        let jaro = pool.execute(move|| {
            let local_v = &v_jaro[..];

            for word in local_v.iter() {
                jaro(&query_jaro, &word);
            }

            tx_jaro.send(vec![""]).unwrap();
        });

        let mut results: Vec<&str> = Vec::new();

        for i in rx.iter().take(2) {
            println!("{:?}", i);
            results.extend(&i);
        }
        let serialized = serde_json::to_string(&metaphone_result).unwrap();
        Ok(Response::with((status::Ok, serialized)))
    };

    router.get("/:query", move |r: &mut Request| handler(r, &v_arc, &meta_dict_arc));
    Iron::new(router).http("localhost:3003").unwrap();

}
