extern crate bson;
extern crate mongodb;

extern crate serde;
extern crate serde_json;

use mongodb::{Client, ThreadedClient};
use mongodb::db::{ThreadedDatabase};
use mongodb::coll::Collection;
use mongodb::coll::options::FindOptions;

use models::*;

pub type TotalRecordsCount = u64;

pub struct QuestionsRepository{
    client: Client
}

impl QuestionsRepository{

    pub fn new(client : Client) -> QuestionsRepository{
        return QuestionsRepository{
            client: client
        }
    }

    fn coll(&self, name : &'static str)->Collection{
        self.client.db("quizzical").collection(name)
    }

    pub fn categories(&self) -> Result<Categories,String>{
        return self
            .coll("questions")
            .distinct("category",None,None)
            .map(|bsons| 
                bsons
                .iter()
                .map(|bson| 
                    bson
                    .as_str()
                    .expect("Unexpected non-string category")
                    .to_owned() 
                )
                .map(|title| Category{title:title})
                .collect()
            )
            .map(|categories| Categories{categories: categories})
            .map_err(|err| format!("{}",err))
    }

    pub fn questions(&self,category: &str, page: u64, size: u64)->Result<(Vec<Question>,TotalRecordsCount),String>{
        
        let filter = doc!{"category":category.clone()};
        
        let count = self
            .coll("questions")
            .count(Some(filter.clone()),None)
            .unwrap_or(0) as u64;

        if count == 0{
            return Ok((vec![],0));
        }

        let mut find_options = FindOptions::new();
        find_options.limit = Some(size as i64);
        find_options.skip = Some(page.saturating_sub(1).checked_mul(size).unwrap_or(0) as i64);

        return self
            .coll("questions")
            .find(Some(filter),Some(find_options))
            .map(|c| {
                let questions = c.map(|d|{
                        bson::from_bson(bson::Bson::Document(d.unwrap()))
                        .unwrap()
                }).collect();

                (questions,count)
            })
            .map_err(|err| format!("{}",err))
    }
}