use polars::prelude::*;

pub struct Connection {
    pub active: bool,
    recent_data: DataFrame,
}

impl Connection {
    pub fn new() -> Self {
        Self {
            active: false,
            recent_data: DataFrame::empty(),
        }
    }

    pub fn recent_data(&self) -> &DataFrame {
        &self.recent_data
    }

    pub fn reset_recent_data(&mut self) {
        self.recent_data = DataFrame::empty();
    }

    pub fn append_data(&mut self, new_data: DataFrame) -> Result<(), PolarsError> {
        self.recent_data = concat_lf_diagonal(
            [self.recent_data.clone().lazy(), new_data.clone().lazy()],
            Default::default(),
        )?
        .collect()?;
        Ok(())
    }

    pub fn discard_older_than(&mut self, duration: chrono::Duration) -> Result<(), PolarsError> {
        self.recent_data = self
            .recent_data
            .clone()
            .lazy()
            .filter(col("time").gt(col("time").max() - lit(duration)))
            .collect()?;
        Ok(())
    }
}
