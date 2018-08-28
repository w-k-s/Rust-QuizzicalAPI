use models::*;
use repositories::*;

pub struct QuestionsService{
    pub repo : Box<QuestionsRepository>
}

impl QuestionsService{
    pub fn new(repo : QuestionsRepository)->QuestionsService{
        return QuestionsService{
            repo : Box::new(repo)
        }
    }

    pub fn categories(&self) -> Result<Categories, String>{
        return self.repo.categories();
    }

    pub fn questions(&self, category: &str, page: u64, size: u64)->Result<(Vec<Question>,TotalRecordsCount),String>{
        return self.repo.questions(category,page,size);
    }
}