use std::{
  borrow::Cow, collections::HashMap, time::{SystemTime, UNIX_EPOCH}
};

use alumet::{
  measurement::{MeasurementBuffer, MeasurementPoint, WrappedMeasurementValue},
  pipeline::{
      elements::{error::TransformError, transform::TransformContext},
      Transform,
  },
  resources::Resource,
};
use anyhow::Context;

pub struct PostProcessingTransform {
    pub(crate) append_unit_to_metric_name: bool,
    pub(crate) use_unit_display_name: bool,
    pub(crate) prefix: Option<String>,
    pub(crate) suffix: Option<String>
}

impl PostProcessingTransform {
    fn prepare_display_name(&mut self, measurements: &mut MeasurementBuffer, ctx: &TransformContext) {
        for m in measurements.iter_mut() {
            log::info!("dealing with {:?} and {:?}", m.metric, m.consumer.kind());
            let full_metric = ctx
                .metrics
                .by_id(&m.metric)
                .unwrap();
            // extract the metric name, appending its unit if configured so
             let metric_name = if self.append_unit_to_metric_name {
                let unit_string = if self.use_unit_display_name {
                    full_metric.unit.display_name()
                } else {
                    full_metric.unit.unique_name()
                };
                if unit_string.is_empty() {
                    full_metric.name.to_owned()
                } else {
                    format!("{}_{}", full_metric.name, unit_string)
                }
            } else {
                full_metric.name.clone()
            };
            // Create the display name based on the previous metric_name and prefix/suffix 
            m.add_attr(Cow::Owned("display_name".to_string()),alumet::measurement::AttributeValue::String(format!("{}{}{}", self.prefix.as_ref().unwrap(), metric_name, self.suffix.as_ref().unwrap())));            
        }
    }
}

impl Transform for PostProcessingTransform {
    fn apply(&mut self, measurements: &mut MeasurementBuffer, _ctx: &TransformContext) -> Result<(), TransformError> {
        self.prepare_display_name(measurements, _ctx);
        Ok(())
    }
}