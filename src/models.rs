extern crate serde;
extern crate serde_derive;
extern crate serde_json;

use serde_derive::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug)]
pub enum ValidationError {
    Constraint { pointer: String, message: String },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ValidationError::Constraint { pointer, message } = self;
        write!(f, "ValidationError{{ {}: {} }}", pointer, message,)
    }
}

#[derive(Serialize, Deserialize, Debug, GraphQLObject)]
pub struct Category {
    pub title: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, GraphQLObject)]
pub struct Choice {
    pub id: Option<i32>,
    pub title: String,
    pub correct: bool,
}

#[derive(Serialize, Deserialize, Debug, GraphQLObject)]
pub struct Question {
    pub id: Option<i32>,
    pub question: String,
    pub category: String,
    pub choices: Vec<Choice>,
}

#[derive(GraphQLInputObject)]
pub struct NewQuestion {
    pub question: String,
    pub category: String,
    pub choices: Vec<NewChoice>,
}

#[derive(GraphQLInputObject)]
pub struct NewChoice {
    pub title: String,
    pub correct: bool,
}

impl Question {
    pub fn validate(question: &Question) -> Result<(), ValidationError> {
        if question
            .choices
            .iter()
            .filter(|choice| choice.correct)
            .count()
            > 1
        {
            return Err(ValidationError::Constraint {
                pointer: "/data/attribute/choices".to_string(),
                message: "Only one correct choice allowed".to_string(),
            });
        }
        Ok(())
    }
}
