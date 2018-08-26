extern crate hyper;
extern crate futures;

#[macro_use]
extern crate mopa;//makes downcasting from T -> Object easier.

#[macro_use(bson, doc)]
extern crate bson;
extern crate mongodb;

use std::sync::Arc;
use std::collections::HashMap;
use std::ops::Deref;

use hyper::{Body, Request, Response, Server};
use hyper::rt::Future;
use hyper::service::service_fn;
use hyper::{Method, StatusCode};

use mongodb::{Client, ThreadedClient};
use mongodb::db::{Database, ThreadedDatabase};
use mongodb::coll::Collection;

use futures::future;

type BoxedResponse = Box<Future<Item=Response<Body>,Error=hyper::Error> + Send>;

const API_ROOT: &'static str = "/api/v3";

fn app(req: Request<Body>, registry: Arc<ControllerRegistry>) -> BoxedResponse {
    let mut response = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            *response.body_mut() = Body::from("Try POSTing data to /echo");
        },
        (&Method::GET, "/api/v3/categories") =>{
            let controller : &CategoriesController = registry.get("Categories").unwrap();
            controller.categories(&req,&mut response)
        },
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        },
    };

    Box::new(future::ok(response))
}

struct QuestionsRepository{
    client: Client
}

impl QuestionsRepository{

    fn new(client : Client) -> QuestionsRepository{
        return QuestionsRepository{
            client: client
        }
    }

    fn coll(&self, name : &'static str)->Collection{
        self.client.db("quizzical").collection(name)
    }

    fn categories(&self) -> Result<Vec<String>,String>{
        return self.coll("questions").distinct("Cat2egory",None,None)
            .map(|bsons| bsons.iter().map(|bson| bson.as_str().expect("Unexpected non-string category").to_owned() ).collect() )
            .map_err(|err| format!("{:?}",err))
    }
}

struct CategoriesService{
    repo : Box<QuestionsRepository>
}

impl CategoriesService{
    fn new(repo : QuestionsRepository)->CategoriesService{
        return CategoriesService{
            repo : Box::new(repo)
        }
    }

    fn categories(&self) -> Result<Vec<String>, String>{
        return self.repo.categories();
    }
}

trait Controller : mopa::Any+Sync+Send {}
mopafy!(Controller);

struct CategoriesController{
    service : Box<CategoriesService>
}

impl CategoriesController{
    fn new(service : CategoriesService)->CategoriesController{
        return CategoriesController{
            service : Box::new(service)
        }
    }

    fn categories(&self, request: &Request<Body>, response: &mut Response<Body>)->(){
        *response.body_mut() = Body::from(self.service.categories().unwrap().join(",").to_string());        
    }
}

impl Controller for CategoriesController{}

struct ControllerRegistry{
    register : HashMap<&'static str, Box<Controller + 'static>>
}

impl ControllerRegistry{
    fn new()-> ControllerRegistry{
        return ControllerRegistry{
            register : HashMap::new()
        }
    }

    fn put<T: Controller + 'static>(&mut self, key: &'static str, controller : T){
        self.register.insert(key,Box::new(controller));
    }

    fn get<T: Controller + 'static>(&self, key: &'static str) -> Option<&T>{
        return self.register.get(key)
                    .and_then(|c| c.downcast_ref::<T>())
    }
}

fn main() {
    let client = Client::connect("localhost", 27017)
        .expect("Failed to initialize standalone client.");

    let repo = QuestionsRepository::new(client);
    let service = CategoriesService::new(repo); 
    let controller = CategoriesController::new(service);
    let mut registry = ControllerRegistry::new();
    registry.put("Categories",controller);

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

	let addr = ([127, 0, 0, 1], 3000).into();
	let server = Server::bind(&addr)
	    .serve(new_service)
	    .map_err(|e| eprintln!("server error: {}", e));

	hyper::rt::run(server);
}
