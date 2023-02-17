use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Config {
    pub mode: String,
    pub node: Uuid,
}

impl Config {
    pub fn init() -> Config {
        let mode = "audioMixer".to_owned();
        let node = Uuid::new_v4();
        Config { mode, node }
    }
}
