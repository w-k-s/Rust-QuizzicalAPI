extern crate futures;
extern crate hyper;
extern crate serde;
extern crate serde_json;
extern crate url;

use juniper::FieldResult;
use models::*;
use repositories::*;
use services::*;

pub struct Context {
    pub categories_service: CategoriesService,
    pub questions_service: QuestionsService,
}

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

graphql_object!(Mutation: Context |&self| {
    field create_category(&executor, name: String) -> FieldResult<Category> {
        
        let category = Category{
            title: name,
        };
        
        let context = executor.context();
        context.categories_service.save_category(&category)?;

        Ok(category)
    }

    field activate_category(&executor, name: String, active: bool) -> FieldResult<SaveCategoryStatus> {
        let context = executor.context();
        let status = context.categories_service.save_category_and_set_active(&name, Some(active))?;
        Ok(status)
    }

    field create_question(&executor, new_question: NewQuestion) -> FieldResult<Question>{
        let context = executor.context();
        let question = context.questions_service.save_question(&Question{
            id: None,
            question: new_question.question,
            category: new_question.category,
            choices: new_question.choices.iter().map(|choice| Choice{
                id: None,
                title: choice.title.clone(),
                correct: choice.correct,
            }).collect(),
        })?;
        Ok(question)
    }
});
