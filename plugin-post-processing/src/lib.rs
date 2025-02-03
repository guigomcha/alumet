mod transform;

use std::borrow::Cow;

use alumet::{
    measurement::{MeasurementBuffer, MeasurementPoint}, metrics::{RawMetricId, TypedMetricId}, pipeline::{
        elements::{error::TransformError, transform::TransformContext},
        Transform,
    }, plugin::{rust::{deserialize_config, serialize_config, AlumetPlugin},
         AlumetPluginStart,
         ConfigTable}
};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use crate::transform::PostProcessingTransform;

pub struct PostProcessingPlugin{
    config: Config,
}

impl AlumetPlugin for PostProcessingPlugin {
    fn name() -> &'static str {
        "post-processing" // the name of your plugin, in lowercase, without the "plugin-" prefix
    }

    fn version() -> &'static str {
        env!("CARGO_PKG_VERSION") // gets the version from the Cargo.toml of the plugin crate
    }

    fn default_config() -> anyhow::Result<Option<ConfigTable>> {
        Ok(Some(serialize_config(Config::default())?))
    }

    fn init(config: ConfigTable) -> anyhow::Result<Box<Self>> {
        let config: Config = deserialize_config(config)?;
        Ok(Box::new(PostProcessingPlugin { config }))
    }

    fn start(&mut self, alumet: &mut AlumetPluginStart) -> anyhow::Result<()> {
        let transform: PostProcessingTransform = PostProcessingTransform {
                append_unit_to_metric_name: self.config.append_unit_to_metric_name,
                use_unit_display_name: self.config.use_unit_display_name,
                prefix: self.config.prefix.clone(),
                suffix: self.config.suffix.clone()
        };

        // Add the transform to the measurement pipeline
        alumet.add_transform(Box::new(transform));
        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        log::info!("Post processing plugin stopped!");
        Ok(())
    }
}


#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Config {
    append_unit_to_metric_name: bool,
    use_unit_display_name: bool,
    prefix: String,
    suffix: String,
    // filter_discard
    // filter_accept
}

impl Default for Config {
    fn default() -> Self {
        Self {
            append_unit_to_metric_name: true,
            use_unit_display_name: true,
            prefix: String::from(""),
            suffix: String::from(""),
            // filter_discard
            // filter_accept
        }
    }
}