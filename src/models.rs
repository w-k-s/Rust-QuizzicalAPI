use serde_derive::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug)]
pub struct ValidationError {
    message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
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
            return Err(ValidationError {
                message: "Only one correct choice allowed".to_string(),
            });
        }
        Ok(())
    }
}
