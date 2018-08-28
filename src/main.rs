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
use std::collections::HashMap;

use hyper::{Body, Request, Response, Server};
use hyper::rt::Future;
use hyper::header::{ HeaderValue,CONTENT_TYPE};
use hyper::service::service_fn;
use hyper::{Method,StatusCode};

use mongodb::{Client, ThreadedClient};
use mongodb::db::{ThreadedDatabase};
use mongodb::coll::Collection;
use mongodb::coll::options::FindOptions;

use futures::future;

use serde::{Serialize};

use url::form_urlencoded;

extern crate pretty_env_logger;

type BoxedResponse = Box<Future<Item=Response<Body>,Error=hyper::Error> + Send>;
type TotalRecordsCount = u64;

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
            ).map_err(|err| format!("{}",err))
    }

    fn questions(&self,category: &str, page: u64, size: u64)->Result<(Vec<Question>,TotalRecordsCount),String>{
        
        let filter = doc!{"category":category.clone()};
        
        let count = self
            .coll("questions")
            .count(Some(filter.clone()),None)
            .unwrap_or(0) as u64;

        if count == 0{
            return Ok((vec![],0));
        }

        let mut find_options = FindOptions::new();
        find_options.limit = Some(size as i64);
        find_options.skip = Some(page.saturating_sub(1).checked_mul(size).unwrap_or(0) as i64);

        return self
            .coll("questions")
            .find(Some(filter),Some(find_options))
            .map(|c| {
                let questions = c.map(|d|{
                        bson::from_bson(bson::Bson::Document(d.unwrap()))
                        .unwrap()
                }).collect();

                (questions,count)
            })
            .map_err(|err| format!("{}",err))
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

    fn questions(&self, category: &str, page: u64, size: u64)->Result<(Vec<Question>,TotalRecordsCount),String>{
        return self.repo.questions(category,page,size);
    }
}

trait Controller : mopa::Any+Sync+Send {}
mopafy!(Controller);

struct QuestionsController{
    service : Box<QuestionsService>
}

#[derive(Serialize, Deserialize, Debug)]
struct PaginatedResponse<T>{
    data : Vec<T>,
    page: u64,
    size: u64,
    page_count: u64,
    last: bool
}

impl <T> PaginatedResponse<T>{

    fn new(data: Vec<T>, page: u64, total_records: u64, limit: u64)->PaginatedResponse<T>{
        let size = data.len() as u64;
        let page_count = (total_records/limit)+1u64;
        let last = page >= page_count;
        return PaginatedResponse{
            data: data,
            page: page,
            size: size,
            page_count: page_count,
            last: last
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Error{
    error : String
}

trait JsonResponse{
    fn json_response<T: Serialize + Sized>(&mut self, status: hyper::StatusCode, body: T);
}

impl JsonResponse for Response<Body>{

    fn json_response<T: Serialize + Sized>(&mut self, status: hyper::StatusCode, body: T){
        self.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        match serde_json::to_string(&body){
            Ok(json) => {
                *self.status_mut() = status;
                *self.body_mut() = Body::from(json);
            },
            Err(err) =>{
                *self.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                *self.body_mut() = Body::from(format!("Serialization error: {}",err));
            }
        }
    }
}

impl QuestionsController{
    fn new(service : QuestionsService)->QuestionsController{
        return QuestionsController{
            service : Box::new(service)
        }
    }

    fn categories(&self, _: &Request<Body>, response: &mut Response<Body>)->(){
        let categories = match self.service.categories(){
            Ok(categories) => categories,
            Err(err) => return response.json_response(StatusCode::INTERNAL_SERVER_ERROR,Error{error: format!("{}",err)})
        };
        return response.json_response(StatusCode::OK, categories);       
    }

    fn questions(&self, request: &Request<Body>, response: &mut Response<Body>)->(){
                
        let params_result = request
        .uri()
        .query()
        .map(|q| q.as_bytes())
        .map(|query| {
            form_urlencoded::parse(query).into_owned().collect::<HashMap<String, String>>()
        });

        let check_category = |params : &HashMap<String,String>|{
            params.get("category").map(|cat| cat.len() ).unwrap_or(0) > 0
        };

        let params = match params_result {
            Some(ref params) if check_category(params) => params,
            _ => return response.json_response(StatusCode::BAD_REQUEST,Error{error: "Required Parameter 'category' not present in query.".to_owned()})
        };

        let category = params.get("category").unwrap();
        let page = params.get("page").and_then(|p| p.parse::<u64>().ok().or(None)).unwrap_or(1);
        let size = params.get("size").and_then(|s| s.parse::<u64>().ok().or(None)).unwrap_or(10);

        let (questions,count) = match self.service.questions(category,page,size){
            Ok(output) => output,
            Err(err) => return response.json_response(StatusCode::INTERNAL_SERVER_ERROR,Error{error: format!("{}",err)})
        };

        let paginated_body = PaginatedResponse::new(questions,page,count,size);
        return response.json_response(StatusCode::OK, paginated_body);    
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
    pretty_env_logger::init();

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

	let addr = ([127, 0, 0, 1], 3001).into();
	let server = Server::bind(&addr)
	    .serve(new_service)
	    .map_err(|e| eprintln!("server error: {}", e));

	hyper::rt::run(server);
}
