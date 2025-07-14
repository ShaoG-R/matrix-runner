use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TestCase {
    pub name: String,
    pub features: String,
    pub no_default_features: bool,
    #[serde(default)]
    pub allow_failure: Vec<String>,
    #[serde(default)]
    pub arch: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct TestMatrix {
    pub cases: Vec<TestCase>,
}
