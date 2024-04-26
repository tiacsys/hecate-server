use hecate_protobuf as proto;
use polars::prelude::*;
use std::time::Duration as StdDuration;


pub trait Frame {
    type Error;
    fn frame(&self) -> Result<DataFrame, Self::Error>;
}

impl Frame for proto::SensorData {
    type Error = PolarsError;

    fn frame(&self) -> Result<DataFrame, PolarsError> {
        df!(
            "time" => self.samples.iter().map(|s| chrono::Duration::from_std(StdDuration::from_secs_f32(s.time)).unwrap()).collect::<Vec<_>>(),
            "acc_x" => self.samples.iter().map(|s| s.acceleration.x).collect::<Vec<_>>(),
            "acc_y" => self.samples.iter().map(|s| s.acceleration.y).collect::<Vec<_>>(),
            "acc_z" => self.samples.iter().map(|s| s.acceleration.z).collect::<Vec<_>>(),
        )
    }
}
