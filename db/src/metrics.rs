use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct InsertionMetrics {
    pub total: i32,
    pub successful: i32,
}
