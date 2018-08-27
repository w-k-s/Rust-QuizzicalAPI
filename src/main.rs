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

use std::sync::Arc;
use std::collections::HashMap;

use hyper::{Body, Request, Response, Server};
use hyper::rt::Future;
use hyper::service::service_fn;
use hyper::{Method, StatusCode};

use mongodb::{Client, ThreadedClient};
use mongodb::db::{ThreadedDatabase};
use mongodb::coll::Collection;
use mongodb::coll::options::FindOptions;

use futures::future;

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

#[derive(Serialize, Deserialize, Debug)]
struct Choice{
    title: String,
    correct: bool
}

#[derive(Serialize, Deserialize, Debug)]
struct Question{
    #[serde(rename = "_id")]
    id: bson::oid::ObjectId,//not ideal
    question: String,
    category: String,
    choices: Vec<Choice>
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
        return self
            .coll("questions")
            .distinct("category",None,None)
            .map(|bsons| 
                bsons
                .iter()
                .map(|bson| 
                    bson
                    .as_str()
                    .expect("Unexpected non-string category")
                    .to_owned() 
                ).collect()
            ).map_err(|err| format!("{:?}",err))
    }

    fn questions(&self,category: &str, page: u64, size: u64)->Result<Vec<Question>,String>{
        let mut find_options = FindOptions::new();
        find_options.limit = Some(size as i64);
        find_options.skip = Some(page.saturating_sub(1).checked_mul(size).unwrap_or(0) as i64);

        let filter = doc!{"category":category.clone()};

        return self
            .coll("questions")
            .find(Some(filter),Some(find_options))
            .map(|c| {
                c.map(|d|{
                        bson::from_bson(bson::Bson::Document(d.unwrap()))
                        .unwrap()
                }).collect()
            })
            .map_err(|err| format!("{:?}",err))
    }
}

struct QuestionsService{
    repo : Box<QuestionsRepository>
}

impl QuestionsService{
    fn new(repo : QuestionsRepository)->QuestionsService{
        return QuestionsService{
            repo : Box::new(repo)
        }
    }

    fn categories(&self) -> Result<Vec<String>, String>{
        return self.repo.categories();
    }

    fn questions(&self, category: &str, page: u64, size: u64)->Result<Vec<Question>,String>{
        return self.repo.questions(category,page,size);
    }
}

trait Controller : mopa::Any+Sync+Send {}
mopafy!(Controller);

struct QuestionsController{
    service : Box<QuestionsService>
}

impl QuestionsController{
    fn new(service : QuestionsService)->QuestionsController{
        return QuestionsController{
            service : Box::new(service)
        }
    }

    fn categories(&self, _: &Request<Body>, response: &mut Response<Body>)->(){
        *response.body_mut() = Body::from(self.service.categories().unwrap().join(",").to_string());        
    }

    fn questions(&self, request: &Request<Body>, response: &mut Response<Body>)->(){
        //*response.body_mut() = Body::from(self.service.questions("Science",1,10).unwrap().join("\n").to_string()));        
        *response.body_mut() = Body::from(format!("{:?}",self.service.questions("Science",1,10).unwrap()));        
    }
}

impl Controller for QuestionsController{}

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

	let addr = ([127, 0, 0, 1], 3000).into();
	let server = Server::bind(&addr)
	    .serve(new_service)
	    .map_err(|e| eprintln!("server error: {}", e));

	hyper::rt::run(server);
}
