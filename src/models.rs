extern crate bson;
extern crate serde;
extern crate serde_json;

#[derive(Serialize, Deserialize, Debug)]
pub struct Category {
    pub title: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Categories {
    pub categories: Vec<Category>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Choice {
    pub title: String,
    pub correct: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Question {
    #[serde(rename = "_id")]
    pub id: bson::oid::ObjectId, //not ideal
    pub question: String,
    pub category: String,
    pub choices: Vec<Choice>,
}
