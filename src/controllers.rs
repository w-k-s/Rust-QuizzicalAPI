extern crate futures;
extern crate hyper;
extern crate mopa; //makes downcasting from T -> Object easier.
extern crate serde;
extern crate serde_json;
extern crate url;

use juniper::FieldResult;
use models::*;
use services::*;

pub struct Context {
    pub categories_service: CategoriesService,
    pub questions_service: QuestionsService,
}

// To make our context usable by Juniper, we have to implement a marker trait.
impl juniper::Context for Context {}

pub struct Query;

graphql_object!(Query: Context |&self| {

    field apiVersion() -> &str {
        "1.0"
    }

    field categories(&executor) -> FieldResult<Vec<Category>> {
        let context = executor.context();
        let categories = context.categories_service.categories()?;
        Ok(categories)
    }

    field questions(&executor, category: String, page: Option<i32>, size: Option<i32>) -> FieldResult<Vec<Question>>{
        let real_page = page.unwrap_or(1);
        let real_size = size.unwrap_or(10);
        
        let context = executor.context();
        let questions = context.questions_service.questions(&category,real_page as i64,real_size as i64)?;
        
        Ok(questions)
    }
});

pub struct Mutation;

graphql_object!(Mutation: Context | &self | {});
