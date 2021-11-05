use serde::Deserialize;

#[derive(Deserialize)]
pub struct CatFactAndPicture {
    pub fact: String,
    pub image_url: String,
}
