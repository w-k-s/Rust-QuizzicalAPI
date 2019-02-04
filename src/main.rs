mod controllers;
mod models;
mod repositories;
mod services;

extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
extern crate tokio_io_pool;
#[macro_use]
extern crate juniper;
#[macro_use]
extern crate juniper_codegen;

extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;

extern crate log;
extern crate simple_logger;
extern crate url;

use std::env;
use std::sync::Arc;

use hyper::rt::Future;
use hyper::service::service_fn;
use hyper::{Body, Response, Server};
use hyper::{Method, StatusCode};
use juniper::RootNode;

use r2d2::Pool;
use r2d2_postgres::{PostgresConnectionManager, TlsMode};

use futures::future;
use futures_cpupool::CpuPool;
use log::{error, info};

use controllers::*;
use repositories::*;
use services::*;

fn main() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();

    let conn_string = env::var("DB_CONN_STRING").expect("invalid DB_CONN_STRING");
    let listen_addr = env::var_os("LISTEN_ADDRESS")
        .map(|addr| addr.into_string().expect("invalid LISTEN_ADDRESS"))
        .unwrap_or("127.0.0.1:3000".to_owned());

    println!("DB_CONN_STRING: {}", conn_string);
    println!("LISTEN_ADDRESS: {}", listen_addr);

    let manager = PostgresConnectionManager::new(conn_string, TlsMode::None).unwrap();
    let pool = Pool::new(manager).unwrap();

    let categories_repository = CategoriesRepository::new(pool.clone());
    let questions_repository = QuestionsRepository::new(pool.clone());

    let categories_service = CategoriesService::new(categories_repository);
    let questions_service = QuestionsService::new(questions_repository);

    let context = Arc::new(Context {
        categories_service: categories_service,
        questions_service: questions_service,
    });

    let cpu_pool = CpuPool::new(4);
    let root_node = Arc::new(RootNode::new(controllers::Query, controllers::Mutation));

    let new_service = move || {
        let cpu_pool = cpu_pool.clone();
        let root_node = root_node.clone();
        let ctx = context.clone();
        service_fn(move |req| -> Box<Future<Item = _, Error = _> + Send> {
            let cpu_pool = cpu_pool.clone();
            let root_node = root_node.clone();
            let ctx = ctx.clone();
            match (req.method(), req.uri().path()) {
                (&Method::GET, "/") => Box::new(juniper_hyper::graphiql("/graphql")),
                (&Method::GET, "/graphql") => {
                    Box::new(juniper_hyper::graphql(cpu_pool, root_node, ctx, req))
                }
                (&Method::POST, "/graphql") => {
                    Box::new(juniper_hyper::graphql(cpu_pool, root_node, ctx, req))
                }
                _ => {
                    let mut response = Response::new(Body::empty());
                    *response.status_mut() = StatusCode::NOT_FOUND;
                    Box::new(future::ok(response))
                }
            }
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
