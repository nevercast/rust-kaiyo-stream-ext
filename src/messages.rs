use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct SimpleControllerInput {
    pub throttle: f32,
    pub steer: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
    pub jump: bool,
    pub boost: bool,
    pub handbrake: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct SelectionMessage {
    pub model: String,
    pub actions: Option<SimpleControllerInput>,
}

#[derive(Serialize, Debug, Clone)]
pub struct StatisticsMessage {
    pub model: String,
    pub counts: u64,
}

#[derive(Serialize, Debug, Clone)]
pub enum Message {
    Selection(SelectionMessage),
    Statistics(StatisticsMessage),
}