extern crate iron;
#[macro_use]
extern crate router;
extern crate nlp;
extern crate serde;
extern crate serde_json;
extern crate scoped_threadpool;
extern crate threadpool;
extern crate promising_future;
use scoped_threadpool::Pool;
use promising_future::future_promise;
use nlp::distance::levenshtein;
use nlp::distance::jaro;
use nlp::phonetics::metaphone::metaphone;
use iron::prelude::*;
use iron::status;
use router::{Router};
use std::io::prelude::*;
use std::fs::File;
use std::path::Path;

// Script de detection de mots
fn main() {
    // Recuperation du dico de mots
    let path = Path::new("words2.txt");
    let mut s = String::new();
    let mut file = File::open(&path).unwrap();
    file.read_to_string(&mut s).unwrap();

    // Création d'un Vector contenant la liste de mots
    let wbyl: Vec<String> = s.lines().map(|s| s.to_owned()).collect();
    // Préproccessing de metaphone pour chaque mots dans le dico et création d'un Vec
    let meta_dict: Vec<(String, String)> = s.clone().lines().map(|s| s.to_owned()).map(|s| (s.to_owned(), metaphone(&s))).collect();

    // Init du router
    let mut router = Router::new();

    // Handler de la request de test
    // Le :query est le mot a tester contre le dico
    fn handler(req: &mut Request,  v: &Vec<String>, meta_dict:&Vec<(String, String)>) -> IronResult<Response> {
        let query = req.extensions.get::<Router>().unwrap().find("query").unwrap_or("/");

        let mut metaphone_result: Vec<&String> = Vec::new();

        // Création du metaphone de l'input
        let metaphone_input = metaphone(&query);

        // Detection des matchs metaphone
        for i in meta_dict {
            if i.1 == metaphone_input {
                metaphone_result.push(&i.0);
            }
        }

        let mut pool = Pool::new(2);
        let (lev_fut, lev_prom) = future_promise();
        let (jaro_fut, jaro_prom) = future_promise();
        pool.scoped(|scope| {
            scope.execute(|| {
                for i in v {
                    let p:&str = &i;
                    levenshtein(&query, &p);
                }
                lev_prom.set("lev-foo".to_string());
            });
            scope.execute(|| {
                for word in v {
                    let p:&str = &word;
                    jaro(&query, &p);
                }
                jaro_prom.set("lev-bar".to_string());
            });
        });
        let results:Vec<String> = promising_future::all(vec!(lev_fut, jaro_fut)).value().unwrap();

        // Création de la réponse JSON
        let serialized = serde_json::to_string(&results).unwrap();
        Ok(Response::with((status::Ok, serialized)))
    };

    router.get("/:query", move |r: &mut Request| handler(r, &wbyl, &meta_dict));
    Iron::new(router).http("localhost:3003").unwrap();
}
