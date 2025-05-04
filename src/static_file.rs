#[derive(Debug)]
pub struct StaticFile {
    pub path: String,
    pub contents: Vec<u8>,
}
