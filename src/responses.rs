extern crate hyper;
extern crate serde;
extern crate serde_json;

use hyper::header::{HeaderValue, CONTENT_TYPE};
use hyper::StatusCode;
use hyper::{Body, Response};

use serde::Serialize;

#[derive(Serialize, Deserialize, Debug)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub page: u64,
    pub size: u64,
    pub page_count: u64,
    pub last: bool,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, page: u64, total_records: u64, limit: u64) -> PaginatedResponse<T> {
        let size = data.len() as u64;
        let page_count = (total_records / limit) + 1u64;
        let last = page >= page_count;
        return PaginatedResponse {
            data: data,
            page: page,
            size: size,
            page_count: page_count,
            last: last,
        };
    }
}

pub trait JsonResponse {
    fn json_response<T: Serialize + Sized>(&mut self, status: hyper::StatusCode, body: T);
}

impl JsonResponse for Response<Body> {
    fn json_response<T: Serialize + Sized>(&mut self, status: hyper::StatusCode, body: T) {
        self.headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        match serde_json::to_string(&body) {
            Ok(json) => {
                *self.status_mut() = status;
                *self.body_mut() = Body::from(json);
            }
            Err(err) => {
                *self.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                *self.body_mut() = Body::from(format!("Serialization error: {}", err));
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Error {
    pub error: String,
}
