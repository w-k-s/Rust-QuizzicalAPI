mod controllers;
mod models;
mod registry;
mod repositories;
mod responses;
mod services;

extern crate futures;
extern crate hyper;
extern crate tokio_io_pool;

#[macro_use]
extern crate mopa; //makes downcasting from T -> Object easier.

#[macro_use(bson, doc)]
extern crate bson;
extern crate mongodb;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

extern crate log;
extern crate simple_logger;
extern crate url;

use std::env;
use std::sync::Arc;

use hyper::rt::Future;
use hyper::service::service_fn;
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};

use futures::future;
use log::{error, info};
use mongodb::db::ThreadedDatabase;
use mongodb::{Client, ThreadedClient};

use controllers::*;
use registry::*;
use repositories::*;
use services::*;

type BoxedResponse = Box<Future<Item = Response<Body>, Error = hyper::Error> + Send>;

fn app(req: Request<Body>, registry: Arc<ControllerRegistry>) -> BoxedResponse {
    let mut response = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            *response.body_mut() = Body::from("Try GETing data from /api/v3/categories");
        }
        (&Method::GET, "/api/v3/categories") => {
            let controller: &QuestionsController = registry.get("Questions").unwrap();
            info!("Handling with controller: {:p}", controller);
            controller.categories(&req, &mut response)
        }
        (&Method::GET, "/api/v3/questions") => {
            let controller: &QuestionsController = registry.get("Questions").unwrap();
            controller.questions(&req, &mut response)
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };

    Box::new(future::ok(response))
}

fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
    let db_host = env::var_os("DB_HOST")
        .map(|host| host.into_string().expect("invalid DB_HOST"))
        .unwrap_or("localhost".to_owned());
    let db_port = env::var_os("DB_PORT")
        .map(|port| port.into_string().expect("invalid DB_PORT"))
        .map(|port| port.parse::<u16>().expect("invalid DB_PORT"))
        .unwrap_or(27017);
    let db_name = env::var_os("DB_NAME")
        .map(|host| host.into_string().expect("invalid DB_NAME"))
        .unwrap_or("quizzical".to_owned());
    let db_user = env::var_os("DB_USER").map(|host| host.into_string().expect("invalid DB_USER"));
    let db_pass = env::var_os("DB_PASS").map(|host| host.into_string().expect("invalid DB_NAME"));
    let listen_addr = env::var_os("LISTEN_ADDRESS")
        .map(|addr| addr.into_string().expect("invalid LISTEN_ADDRESS"))
        .unwrap_or("127.0.0.1:3000".to_owned());

    println!("DB_HOST: {}", db_host);
    println!("DB_NAME: {}", db_name);
    println!("DB_PORT: {}", db_port);
    println!("DB_USER: {:?}", db_user);
    println!("DB_PASS: {:?}", db_pass);
    println!("LISTEN_ADDRESS: {}", listen_addr);

    let client =
        Client::connect(&db_host, db_port).expect("Failed to initialize standalone client.");
    if let (Some(username), Some(password)) = (db_user, db_pass) {
        let db = client.db(&db_name);
        let _ = db.auth(&username, &password);
    }

    let repo = QuestionsRepository::new(client, &db_name);
    let service = QuestionsService::new(repo);
    let controller = QuestionsController::new(service);
    let mut registry = ControllerRegistry::new();
    registry.put("Questions", controller);

    let registry_ref = Arc::new(registry);
    //move registry_ref into new_service closure so that new_service closure will own registry.
    let new_service = move || {
        // Create a copy of registry_ref to pass to pass into service functions. Is this necessary? Is this right?
        let registry_ref = registry_ref.clone();
        service_fn(move |request| {
            //each connection gets a new service. share arc reference to registry with service.
            info!("{} {}", request.method().clone(), request.uri().clone());
            app(request, registry_ref.clone())
        })
    };

    let addr = listen_addr.parse().unwrap();
    let server = Server::bind(&addr)
        .serve(new_service)
        .map_err(|e| eprintln!("server error: {}", e));

    // `hyper::rt::run` internally uses `tokio::run`.
    // According to the [documentation](https://docs.rs/tokio-io-pool/0.1.5/tokio_io_pool/) of tokio-io-pool,
    // tokio::run shares one reactor across multiple requests.
    //
    // Using jmeter, I simulated 1000 requests ramped over 2 seconds with a loop count of 2.
    // This resulted in most requests being dropped.
    //
    // `tokio-io-pool` spawns a new thread from a pool for each request and request has its own reactor.
    // After replacing `hyper::rt::run` with `tokio_io_pool::run` and rerunning the same jmeter simulation, All requests succeeded!
    // Not a single request was dropped.

    tokio_io_pool::run(server);
}
