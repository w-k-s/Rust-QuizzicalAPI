use juniper::FieldResult;
use models::*;
use repositories::*;
use serde_derive::{Deserialize, Serialize};
use services::*;

#[derive(Serialize, Deserialize, Debug, GraphQLObject)]
pub struct PaginatedQuestions {
    pub data: Vec<Question>,
    pub page: i32,
    pub size: i32,
    pub page_count: i32,
    pub last: bool,
}

impl PaginatedQuestions {
    pub fn new(
        data: Vec<Question>,
        page: i32,
        total_records: i32,
        mut limit: i32,
    ) -> PaginatedQuestions {
        if limit <= 0 {
            limit = 1;
        }

        let page_count = (total_records as f64 / limit as f64).ceil() as i32;

        let size = data.len() as i32;
        let last = page >= page_count;
        return PaginatedQuestions {
            data: data,
            page: page,
            size: size,
            page_count: page_count,
            last: last,
        };
    }
}

pub struct Context {
    pub categories_service: CategoriesService,
    pub questions_service: QuestionsService,
    pub authorization_service: AuthorizationService,
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

    field questions(&executor, category: String, page: Option<i32>, size: Option<i32>) -> FieldResult<PaginatedQuestions>{
        let real_page = page.unwrap_or(1);
        let real_size = size.unwrap_or(10);
        
        let context = executor.context();
        let total_records = context.questions_service.count_questions(&category)? as i32;
        let questions = context.questions_service.questions(&category,real_page as i64,real_size as i64)?;
        
        Ok(PaginatedQuestions::new(questions, real_page, total_records , real_size))
    }
});

pub struct Mutation;

graphql_object!(Mutation: Context |&self| {
    field create_category(&executor, name: String, digest: String) -> FieldResult<Category> {
        let context = executor.context();
        context.authorization_service.verify_digest(&digest,"GET","/graphql")?;

        let category = Category{
            title: name,
        };
        
        context.categories_service.save_category(&category)?;

        Ok(category)
    }

    field activate_category(&executor, name: String, active: bool, digest: String) -> FieldResult<SaveCategoryStatus> {
        let context = executor.context();
        context.authorization_service.verify_digest(&digest,"GET","/graphql")?;

        let status = context.categories_service.save_category_and_set_active(&name, Some(active))?;
        Ok(status)
    }

    field create_question(&executor, new_question: NewQuestion, digest: String) -> FieldResult<Question>{
        let context = executor.context();
        context.authorization_service.verify_digest(&digest,"GET","/graphql")?;

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
