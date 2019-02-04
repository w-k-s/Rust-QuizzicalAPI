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

    pub fn save_category(
        &self,
        category: &Category,
    ) -> Result<SaveCategoryStatus, RepositoryError> {
        return (*self.repo).save_category(category);
    }

    pub fn save_category_and_set_active(
        &self,
        category: &str,
        active: Option<bool>,
    ) -> Result<SaveCategoryStatus, RepositoryError> {
        return (*self.repo).save_category_and_set_active(category, active);
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

    pub fn save_question(&self, question: &Question) -> Result<Question, RepositoryError> {
        return (*self.repo).save_question(question);
    }

    pub fn count_questions(&self, category: &str) -> Result<i64, RepositoryError> {
        return (*self.repo).count_questions(category);
    }
}
