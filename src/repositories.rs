use log::{error, info};
use models::{Category, Choice, Question};
use postgres::rows::Rows;
use postgres::transaction::Transaction;
use postgres::types::ToSql;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, GraphQLEnum)]
pub enum SaveCategoryStatus {
    Created,
    Exists,
}

pub struct Connection {
    pub pool: Pool<PostgresConnectionManager>,
}

impl Connection {
    fn execute(&self, query: &str, params: &[&ToSql]) -> Result<u64, RepositoryError> {
        return self
            .pool
            .get()
            .map_err(|e| e.into())
            .and_then(|c| c.execute(query, params).map_err(|e| e.into()));
    }

    fn query(&self, query: &str, params: &[&ToSql]) -> Result<Rows, RepositoryError> {
        return self
            .pool
            .get()
            .map_err(|e| e.into())
            .and_then(|c| c.query(query, params).map_err(|e| e.into()));
    }

    fn transaction<T, F>(&self, do_transaction: F) -> Result<T, RepositoryError>
    where
        F: FnOnce(Transaction) -> Result<T, RepositoryError>,
    {
        return self.pool.get().map_err(|e| e.into()).and_then(|c| {
            c.transaction()
                .map_err(|e| e.into())
                .and_then(|t| do_transaction(t))
        });
    }
}

#[derive(Debug)]
pub enum RepositoryError {
    ConnectionError(String),
    DatabaseError(String, String),
    IOError(String),
    ConversionError(String),
    UnknownError(Option<String>),
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message = match *self {
            RepositoryError::ConnectionError(ref message) => message,
            RepositoryError::DatabaseError(_, ref message) => message,
            RepositoryError::IOError(ref message) => message,
            RepositoryError::ConversionError(ref message) => message,
            RepositoryError::UnknownError(Some(ref message)) => message,
            RepositoryError::UnknownError(None) => "Unknown Error",
        };
        write!(f, "{}", message)
    }
}

impl std::convert::From<r2d2::Error> for RepositoryError {
    fn from(error: r2d2::Error) -> Self {
        return RepositoryError::ConnectionError(format!("{}", error));
    }
}

impl std::convert::From<postgres::Error> for RepositoryError {
    fn from(error: postgres::Error) -> Self {
        if let Some(connection_error) = error.as_connection() {
            return RepositoryError::ConnectionError(format!("{}", connection_error));
        }

        if let Some(db_error) = error.as_db() {
            return RepositoryError::DatabaseError(
                db_error.code.code().into(),
                db_error.message.clone(),
            );
        }

        if let Some(conversion_error) = error.as_conversion() {
            return RepositoryError::ConversionError(format!("{}", conversion_error));
        }

        if let Some(io_error) = error.as_io() {
            return RepositoryError::IOError(format!("{}", io_error));
        }

        return RepositoryError::UnknownError(Some(format!("{}", error)));
    }
}

pub struct CategoriesRepository {
    pub conn: Connection,
}

impl CategoriesRepository {
    pub fn new(pool: Pool<PostgresConnectionManager>) -> CategoriesRepository {
        return CategoriesRepository {
            conn: Connection { pool: pool },
        };
    }

    pub fn save_category(
        &self,
        category: &Category,
    ) -> Result<SaveCategoryStatus, RepositoryError> {
        self.save_category_and_set_active(&category.title, None)
    }

    pub fn save_category_and_set_active(
        &self,
        category: &str,
        active: Option<bool>,
    ) -> Result<SaveCategoryStatus, RepositoryError> {
        info!("save_category(category: '{:?}').", category);

        let (field_names, value_placeholders, on_conflict, values) = match active {
            Some(x) => (
                "(name,active)",
                "($1,$2)",
                "ON CONFLICT(name) DO UPDATE SET active=$2",
                vec![&category as &ToSql, &active as &ToSql],
            ),
            None => (
                "(name)",
                "($1)",
                "ON CONFLICT DO NOTHING",
                vec![&category as &ToSql],
            ),
        };

        let query_string = &format!(
            "INSERT INTO categories {} VALUES {} {};",
            field_names, value_placeholders, on_conflict
        );

        let affected_rows = self.conn.execute(query_string, values.as_slice())?;

        info!(
            "Inserting category suceeded with affected rows '{:?}'.",
            affected_rows
        );

        Ok(match affected_rows {
            x if x > 0u64 => SaveCategoryStatus::Created,
            _ => SaveCategoryStatus::Exists,
        })
    }

    pub fn list_categories(&self) -> Result<Vec<Category>, RepositoryError> {
        let rows = &self
            .conn
            .query("SELECT name FROM categories WHERE active = true", &[])?;

        let mut categories: Vec<Category> = Vec::with_capacity(rows.len());

        for row in rows {
            categories.push(Category { title: row.get(0) });
        }

        Ok(categories)
    }

    pub fn set_category_active(&self, name: &str, active: bool) -> Result<bool, RepositoryError> {
        let affected_rows = self.conn.execute(
            "UPDATE categories SET active = $1 WHERE name = $2 AND active != $1",
            &[&active, &name],
        )?;

        Ok(affected_rows > 0u64)
    }
}

pub struct QuestionsRepository {
    pub conn: Connection,
}

impl QuestionsRepository {
    pub fn new(pool: Pool<PostgresConnectionManager>) -> QuestionsRepository {
        return QuestionsRepository {
            conn: Connection { pool: pool },
        };
    }

    pub fn save_question(&self, question: &Question) -> Result<Question, RepositoryError> {
        info!("save_question(question: '{:?}').", question);

        return self.conn.transaction(|trans| {
            info!("Inserting question '{:?}' into database.", question);

            let id_rows = &trans
                .query(
                    "INSERT INTO questions (text, category) VALUES ($1, $2) RETURNING id",
                    &[&question.question, &question.category],
                )
                .or_else(|e| {
                    error!(
                        "Insert question failed for question: '{:?}', with reason: '{:?}'.",
                        question, e
                    );
                    //rollback will happen when transaction is dropped (i.e. Destructor)
                    trans.set_rollback();
                    Err(e)
                })?;

            info!(
                "Insert question succeeded for question: '{:?}', with updated rows: '{:?}'.",
                question, id_rows,
            );

            let question_id: i64 = id_rows
                .iter()
                .next()
                .and_then(|row| row.get(0))
                .ok_or(RepositoryError::UnknownError(Some(
                    "Failed to get question id".into(),
                )))
                .map_err(|e| {
                    error!(
                        "Insert question succeeded but no id received for question: '{:?}'.",
                        question
                    );
                    trans.set_rollback();
                    e
                })?;

            //Since we don't know how many choices a question has, we need to build a query string for bulk insert manually.

            //value_placeholders refers to the `($1, $2)` part of the query.
            let mut value_placeholders: Vec<String> = vec![];
            //total is the number of fields to be inserted per choice multiplied by the number of choices
            let num_fields = 3;
            let total = num_fields * question.choices.len();

            for i in (0..total).step_by(num_fields) {
                value_placeholders.push(format!("(${}, ${}, ${})", i + 1, i + 2, i + 3))
            }

            //join all the value placeholders i.e. ($1,$2), ($3,$4)
            let joined_value_placeholders = value_placeholders.join(",");

            let query_string = &format!(
                "INSERT INTO choices (question_id, text, correct) VALUES {} RETURNING id",
                joined_value_placeholders
            );

            let mut values: Vec<&ToSql> = vec![];
            for choice in question.choices.iter() {
                values.push(&question_id);
                values.push(&choice.title);
                values.push(&choice.correct);
            }

            info!(
                "Will insert choices for question id '{}' using query '{}' and values '{:?}'.",
                question_id, query_string, values
            );

            let rows: Rows = trans.query(query_string, values.as_slice()).or_else(|e| {
                error!(
                    "Bulk insert choices failed for question_id: '{}', reason: {}.",
                    question_id, e
                );
                //rollback will happen when transaction is dropped (i.e. Destructor)
                trans.set_rollback();
                Err(e)
            })?;;

            // Create a new vector of choices, with the id field set.
            let ids: Vec<i64> = rows.iter().map(|row| row.get(0)).collect();
            let choices_with_ids = question
                .choices
                .iter()
                .zip(ids.iter())
                .map(|choice_id_tuple| {
                    let choice = choice_id_tuple.0;
                    let id = choice_id_tuple.1;
                    Choice {
                        id: Some(*id as i32),
                        title: choice.title.clone(),
                        correct: choice.correct,
                    }
                })
                .collect();

            trans.set_commit();

            trans
                .finish()
                .map_err(|e| {
                    error!(
                        "Finishing insert question failed for question_id '{}' with reason '{}'.",
                        question_id, e
                    );
                    e.into()
                })
                .and(Ok(Question {
                    id: Some(question_id as i32),
                    question: question.question.clone(),
                    category: question.category.clone(),
                    choices: choices_with_ids,
                }))
        });
    }

    pub fn count_questions(&self, category: &str) -> Result<i64, RepositoryError> {
        let count_rows = &self
            .conn
            .query(
                "SELECT COUNT(q.id) FROM questions q INNER JOIN categories c ON c.name = q.category WHERE c.name = $1 AND c.active = TRUE",
                &[&category],
            )
            .map_err(|e| {
                error!(
                    "Error counting questions for category '{}': {}",
                    category, e
                );
                e
            })?;

        let count: i64 = match count_rows.is_empty() {
            true => 0i64,
            false => count_rows.get(0).get(0),
        };

        Ok(count)
    }

    pub fn get_questions(
        &self,
        category: &str,
        page: i64,
        size: i64,
    ) -> Result<Vec<Question>, RepositoryError> {
        let offset = match page {
            0 => 0i64,
            _ => (page - 1i64) * size,
        };

        let question_rows = &self
            .conn
            .query(
                "SELECT q.id,q.text FROM questions q INNER JOIN categories c ON c.name = q.category WHERE c.name = $1 AND c.active = TRUE LIMIT $2 OFFSET $3",
                &[&category, &size, &offset],
            )
            .map_err(|e| {
                error!("Error loading questions for category '{}': {}", category, e);
                e
            })?;

        if question_rows.is_empty() {
            return Ok(vec![]);
        }

        let mut question_ids: Vec<i64> = vec![];
        for question_row in question_rows {
            let id: i64 = question_row.get(0);
            question_ids.push(id);
        }

        let choices_rows = &self
            .conn
            .query(
                "SELECT id,text,correct,question_id FROM choices WHERE question_id = ANY($1)",
                &[&question_ids],
            )
            .map_err(|e| {
                error!(
                    "Error loading choices for questions '{:?}': {}",
                    question_ids, e
                );
                e
            })?;

        let mut choices_map: HashMap<i64, Vec<Choice>> = HashMap::new();
        for choice_row in choices_rows {
            let question_id: i64 = choice_row.get(3);
            let choice_id: i64 = choice_row.get(0);
            let choice = Choice {
                id: Some(choice_id as i32),
                title: choice_row.get(1),
                correct: choice_row.get(2),
            };

            if let Some(mut choices) = choices_map.get_mut(&question_id) {
                choices.push(choice);
                continue;
            }

            choices_map.insert(question_id, vec![choice]);
        }

        let mut questions: Vec<Question> = Vec::with_capacity(question_rows.len());
        for question_row in question_rows {
            let id: i64 = question_row.get(0);
            let text: String = question_row.get(1);
            let choices: Vec<Choice> = choices_map.get(&id).unwrap_or(&vec![]).to_vec();

            questions.push(Question {
                id: Some(id as i32),
                question: text,
                category: category.to_string(),
                choices: choices,
            });
        }

        Ok(questions)
    }
}
