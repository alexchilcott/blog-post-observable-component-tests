use serde::Deserialize;

#[derive(Deserialize)]
pub struct CatFactAndImageUrl {
    pub fact: String,
    pub image_url: String,
}
