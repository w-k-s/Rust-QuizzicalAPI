use models::*;
use repositories::*;
use std::sync::Arc;

pub struct CategoriesService {
    pub repo: Arc<CategoriesRepository>,
}

impl CategoriesService {
    pub fn new(repo: CategoriesRepository) -> CategoriesService {
        return CategoriesService {
            repo: Arc::new(repo),
        };
    }

    pub fn categories(&self) -> Result<Vec<Category>, RepositoryError> {
        return (*self.repo).list_categories();
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
    ) -> Result<Vec<Question>, RepositoryError> {
        return (*self.repo).get_questions(category, page, size);
    }

    pub fn count_questions(&self, category: &str) -> Result<i64, RepositoryError> {
        return (*self.repo).count_questions(category);
    }
}
