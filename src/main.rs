mod models;
mod repositories;
mod services;
mod controllers;
mod responses;
mod registry;

extern crate hyper;
extern crate futures;

#[macro_use]
extern crate mopa;//makes downcasting from T -> Object easier.

#[macro_use(bson, doc)]
extern crate bson;
extern crate mongodb;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

extern crate url;

use std::sync::Arc;
use std::env;

use hyper::{Body, Request, Response, Server};
use hyper::rt::Future;
use hyper::service::service_fn;
use hyper::{Method,StatusCode};

use mongodb::{Client, ThreadedClient};
use futures::future;

use repositories::*;
use services::*;
use controllers::*;
use registry::*;

extern crate pretty_env_logger;

type BoxedResponse = Box<Future<Item=Response<Body>,Error=hyper::Error> + Send>;

fn app(req: Request<Body>, registry: Arc<ControllerRegistry>) -> BoxedResponse {
    let mut response = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            *response.body_mut() = Body::from("Try POSTing data to /echo");
        },
        (&Method::GET, "/api/v3/categories") =>{
            let controller : &QuestionsController = registry.get("Questions").unwrap();
            controller.categories(&req,&mut response)
        },
        (&Method::GET, "/api/v3/questions") =>{
            let controller : &QuestionsController = registry.get("Questions").unwrap();
            controller.questions(&req,&mut response)
        },
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        },
    };

    Box::new(future::ok(response))
}

fn main() {
    pretty_env_logger::init();

    for argument in env::args() {
        println!("{}", argument);
    }

    let db_host = env::var_os("DB_HOST")
                    .map(|host| host.into_string().expect("invalid DB_HOST"))
                    .unwrap_or("localhost".to_owned());
    let db_port = env::var_os("DB_PORT")
                    .map(|port| port.into_string().expect("invalid DB_PORT"))
                    .map(|port| port.parse::<u16>().expect("invalid DB_PORT"))
                    .unwrap_or(27017);
    let port = env::var_os("PORT")
                    .map(|port| port.into_string().expect("invalid PORT"))
                    .map(|port| port.parse::<u16>().expect("invalid PORT"))
                    .unwrap_or(3001);

    let client = Client::connect(&db_host, db_port)
        .expect("Failed to initialize standalone client.");

    let repo = QuestionsRepository::new(client);
    let service = QuestionsService::new(repo); 
    let controller = QuestionsController::new(service);
    let mut registry = ControllerRegistry::new();
    registry.put("Questions",controller);

    let registry_ref = Arc::new(registry);
    //move registry_ref into new_service closure so that new_service closure will own registry.
    let new_service = move || {
        // Create a copy of registry_ref to pass to pass into service functions. Is this necessary? Is this right?
        let registry_ref = registry_ref.clone();
        service_fn(move |request|{
            //each connection gets a new service. share arc reference to registry with service.
            app(request, registry_ref.clone())
        })
    };

	let addr = ([127, 0, 0, 1], port).into();
	let server = Server::bind(&addr)
	    .serve(new_service)
	    .map_err(|e| eprintln!("server error: {}", e));

	hyper::rt::run(server);
}
