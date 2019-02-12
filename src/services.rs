use md5::{Digest, Md5};
use models::*;
use repositories::*;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use uuid::Uuid;

pub enum ServiceError {
    ValidationError(ValidationError),
    RepositoryError(RepositoryError),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ServiceError::ValidationError(e) => write!(f, "{}", e),
            ServiceError::RepositoryError(e) => write!(f, "{}", e),
        }
    }
}

impl std::convert::From<ValidationError> for ServiceError {
    fn from(error: ValidationError) -> Self {
        return ServiceError::ValidationError(error);
    }
}

impl std::convert::From<RepositoryError> for ServiceError {
    fn from(error: RepositoryError) -> Self {
        return ServiceError::RepositoryError(error);
    }
}

pub struct CategoriesService {
    pub repo: Arc<CategoriesRepository>,
}

impl CategoriesService {
    pub fn new(repo: CategoriesRepository) -> CategoriesService {
        return CategoriesService {
            repo: Arc::new(repo),
        };
    }

    pub fn categories(&self) -> Result<Vec<Category>, ServiceError> {
        return (*self.repo).list_categories().map_err(|e| e.into());
    }

    pub fn save_category(&self, category: &Category) -> Result<SaveCategoryStatus, ServiceError> {
        return (*self.repo).save_category(category).map_err(|e| e.into());
    }

    pub fn save_category_and_set_active(
        &self,
        category: &str,
        active: Option<bool>,
    ) -> Result<SaveCategoryStatus, ServiceError> {
        return (*self.repo)
            .save_category_and_set_active(category, active)
            .map_err(|e| e.into());
    }
}

pub struct QuestionsService {
    pub repo: Arc<QuestionsRepository>,
}

impl QuestionsService {
    pub fn new(repo: QuestionsRepository) -> QuestionsService {
        return QuestionsService {
            repo: Arc::new(repo),
        };
    }

    pub fn questions(
        &self,
        category: &str,
        page: i64,
        size: i64,
    ) -> Result<Vec<Question>, ServiceError> {
        return (*self.repo)
            .get_questions(category, page, size)
            .map_err(|e| e.into());;
    }

    pub fn save_question(&self, question: &Question) -> Result<Question, ServiceError> {
        Question::validate(question).map_err(|e| ServiceError::from(e))?;
        return (*self.repo).save_question(question).map_err(|e| e.into());;
    }

    pub fn count_questions(&self, category: &str) -> Result<i64, ServiceError> {
        return (*self.repo).count_questions(category).map_err(|e| e.into());;
    }
}

pub enum AuthorizationError {
    InvalidFormat,
    MissingField { field: String },
    IncorrectResponse,
}

impl std::fmt::Display for AuthorizationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AuthorizationError::InvalidFormat => write!(f, "{}", "Expected digest format: 'Digest username=\"?\", realm=\"?\", nonce=\"?\", opaque=\"?\", uri=\"?\", response=\"?\"'."),
            AuthorizationError::MissingField{field} => write!(f, "Digest does not contain required field: '{}'", field),
            AuthorizationError::IncorrectResponse => write!(f, "{}", "The 'response' field is incorrect or expired."),
        }
    }
}

pub struct AuthorizationService {
    admin_username: String,
    admin_password: String,
}

impl AuthorizationService {
    pub fn new(admin_username: &str, admin_password: &str) -> Self {
        return AuthorizationService {
            admin_username: admin_username.into(),
            admin_password: admin_password.into(),
        };
    }

    fn hashed_unique_id(&self) -> String {
        let uuid = Uuid::new_v4();
        Self::md5(format!("{}", uuid))
    }

    fn md5(input: String) -> String {
        let mut hasher = Md5::default();
        hasher.input(input);
        format!("{:x}", hasher.result())
    }

    fn nonce(&self) -> String {
        self.hashed_unique_id()
    }

    fn opaque(&self) -> String {
        self.hashed_unique_id()
    }

    fn realm(&self) -> String {
        "administrator".into()
    }

    pub fn www_authenticate(&self) -> String {
        format!(
            "Digest realm=\"{}\", nonce=\"{}\", opaque=\"{}\"",
            self.realm(),
            self.nonce(),
            self.opaque()
        )
    }

    pub fn verify_digest(
        &self,
        raw_digest: &str,
        request_method: &str,
        uri: &str,
    ) -> Result<(), AuthorizationError> {
        let clean_digest = raw_digest.trim();
        let prefix = "Digest ";

        if !clean_digest.starts_with(prefix) {
            return Err(AuthorizationError::InvalidFormat);
        }

        let (_, fields_string) = clean_digest.split_at(prefix.len());
        let key_values: HashMap<&str, &str> = fields_string
            .split(',')
            .map(|field| {
                let pair: Vec<&str> = field.trim().split('=').collect();
                let key = pair[0].trim();
                let value = pair.get(1).unwrap_or(&"").trim().trim_matches('\"');
                (key, value)
            })
            .collect();

        let nonce = key_values
            .get("nonce")
            .ok_or(AuthorizationError::MissingField {
                field: "nonce".into(),
            })?;
        let response = *key_values
            .get("response")
            .ok_or(AuthorizationError::MissingField {
                field: "response".into(),
            })?;

        let a1 = Self::md5(format!(
            "{}:{}:{}",
            self.admin_username,
            self.realm(),
            self.admin_password
        ));
        let a2 = Self::md5(format!("{}:{}", request_method, uri));
        let expected_response = Self::md5(format!("{}:{}:{}", a1, nonce, a2));

        if expected_response == response {
            Ok(())
        } else {
            Err(AuthorizationError::IncorrectResponse)
        }
    }
}
