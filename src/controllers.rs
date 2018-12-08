extern crate futures;
extern crate hyper;
extern crate mopa; //makes downcasting from T -> Object easier.
extern crate serde;
extern crate serde_json;
extern crate url;

use std::collections::HashMap;

use hyper::StatusCode;
use hyper::{Body, Request, Response};
use url::form_urlencoded;

use responses::*;
use services::*;

pub trait Controller: mopa::Any + Sync + Send {}
mopafy!(Controller);

pub struct QuestionsController {
    service: Box<QuestionsService>,
}

impl QuestionsController {
    pub fn new(service: QuestionsService) -> QuestionsController {
        return QuestionsController {
            service: Box::new(service),
        };
    }

    pub fn categories(&self, _: &Request<Body>, response: &mut Response<Body>) -> () {
        let categories = match self.service.categories() {
            Ok(categories) => categories,
            Err(err) => {
                return response.json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Error {
                        error: format!("{}", err),
                    },
                )
            }
        };
        return response.json_response(StatusCode::OK, categories);
    }

    pub fn questions(&self, request: &Request<Body>, response: &mut Response<Body>) -> () {
        let params_result = request.uri().query().map(|q| q.as_bytes()).map(|query| {
            form_urlencoded::parse(query)
                .into_owned()
                .collect::<HashMap<String, String>>()
        });

        let check_category = |params: &HashMap<String, String>| {
            params.get("category").map(|cat| cat.len()).unwrap_or(0) > 0
        };

        let params = match params_result {
            Some(ref params) if check_category(params) => params,
            _ => {
                return response.json_response(
                    StatusCode::BAD_REQUEST,
                    Error {
                        error: "Required Parameter 'category' not present in query.".to_owned(),
                    },
                )
            }
        };

        let category = params.get("category").unwrap();
        let page = params
            .get("page")
            .and_then(|p| p.parse::<u64>().ok().or(None))
            .unwrap_or(1);
        let size = params
            .get("size")
            .and_then(|s| s.parse::<u64>().ok().or(None))
            .unwrap_or(10);

        let (questions, count) = match self.service.questions(category, page, size) {
            Ok(output) => output,
            Err(err) => {
                return response.json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Error {
                        error: format!("{}", err),
                    },
                )
            }
        };

        let paginated_body = PaginatedResponse::new(questions, page, count, size);
        return response.json_response(StatusCode::OK, paginated_body);
    }
}

impl Controller for QuestionsController {}
