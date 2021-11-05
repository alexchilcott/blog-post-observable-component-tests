#[derive(Clone)]
pub struct Configuration {
    pub host: String,
    pub port: u16,
    pub cat_images_api_base_url: String,
    pub cat_facts_api_base_url: String,
}
