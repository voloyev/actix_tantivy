extern crate tempdir;

#[macro_use]
extern crate tantivy;
use tantivy::collector::TopCollector;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::Index;
use tempdir::TempDir;

extern crate actix;
extern crate actix_web;
extern crate env_logger;

#[macro_use]
extern crate serde_derive;

use actix_web::{
    http, middleware::Logger, server, App, Form, HttpRequest, HttpResponse, Result, State,
};

struct AppState {
    search: String,
}

fn main() {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();
    let sys = actix::System::new("seach-example");

    let _addr = server::new(|| {
        App::with_state(AppState {
            search: "bar".to_string(),
        })
        .middleware(Logger::default())
        .middleware(Logger::new("%a %{User-Agent}i"))
        .resource("/", |r| {
            r.method(http::Method::GET).with(index);
        })
        .resource("/search", |r| {
            r.method(http::Method::POST).with(post_search)
        })
    })
    .bind("127.0.0.1:8080")
    .expect("Can not bind to 127.0.0.1:8080")
    .start();

    println!("Starting http server: 127.0.0.1:8080");
    let _ = sys.run();
}

fn index(_req: HttpRequest<AppState>) -> Result<HttpResponse> {
    Ok(HttpResponse::build(http::StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/form.html")))
}

#[derive(Deserialize)]
pub struct Search {
    search: String,
}

fn post_search(params: Form<Search>) -> Result<HttpResponse> {
    Ok(HttpResponse::build(http::StatusCode::OK)
        .content_type("application/json")
        .body(format!("{:?}", tantivy(&params.search).0)))
}

fn tantivy(search: &String) -> (String, tantivy::Result<()>) {
    let index_path = TempDir::new("tantivy_example_dir").unwrap();

    let mut schema_builder = SchemaBuilder::default();

    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_text_field("body", TEXT);

    let schema = schema_builder.build();

    let index = Index::create_in_dir(&index_path, schema.clone()).unwrap();

    let mut index_writer = index.writer(50_000_000).unwrap();

    let title = schema.get_field("title").unwrap();
    let body = schema.get_field("body").unwrap();

    let mut old_man_doc = Document::default();
    old_man_doc.add_text(title, "The Old Man and the Sea");
    old_man_doc.add_text(
        body,
        "He was an old man who fished alone in a skiff in the Gulf Stream and \
         he had gone eighty-four days now without taking a fish.",
    );

    index_writer.add_document(old_man_doc);

    index_writer.add_document(doc!(
    title => "Of Mice and Men",
    body => "A few miles south of Soledad, the Salinas River drops in close to the hillside \
             bank and runs deep and green. The water is warm too, for it has slipped twinkling \
             over the yellow sands in the sunlight before reaching the narrow pool. On one \
             side of the river the golden foothill slopes curve up to the strong and rocky \
             Gabilan Mountains, but on the valley side the water is lined with trees—willows \
             fresh and green with every spring, carrying in their lower leaf junctures the \
             debris of the winter’s flooding; and sycamores with mottled, white, recumbent \
             limbs and branches that arch over the pool"
    ));

    index_writer.add_document(doc!(
    title => "Of Mice and Men",
    body => "A few miles south of Soledad, the Salinas River drops in close to the hillside \
             bank and runs deep and green. The water is warm too, for it has slipped twinkling \
             over the yellow sands in the sunlight before reaching the narrow pool. On one \
             side of the river the golden foothill slopes curve up to the strong and rocky \
             Gabilan Mountains, but on the valley side the water is lined with trees—willows \
             fresh and green with every spring, carrying in their lower leaf junctures the \
             debris of the winter’s flooding; and sycamores with mottled, white, recumbent \
             limbs and branches that arch over the pool"
    ));

    // Multivalued field just need to be repeated.
    index_writer.add_document(doc!(
    title => "Frankenstein",
    title => "The Modern Prometheus",
    body => "You will rejoice to hear that no disaster has accompanied the commencement of an \
             enterprise which you have regarded with such evil forebodings.  I arrived here \
             yesterday, and my first task is to assure my dear sister of my welfare and \
             increasing confidence in the success of my undertaking."
    ));

    index_writer.commit().unwrap();

    index.load_searchers().unwrap();

    let searcher = index.searcher();

    let query_parser = QueryParser::for_index(&index, vec![title, body]);
    let query = query_parser.parse_query(&search).unwrap();
    let mut top_collector = TopCollector::with_limit(10);

    searcher.search(&*query, &mut top_collector).unwrap();

    let doc_addresses = top_collector.docs();

    let mut output = String::from("");
    for doc_address in doc_addresses {
        let retrieved_doc = searcher.doc(doc_address).unwrap();
        output += schema.to_json(&retrieved_doc).as_str();
    }

    index_writer.wait_merging_threads().unwrap();

    (output, Ok(()))
}
