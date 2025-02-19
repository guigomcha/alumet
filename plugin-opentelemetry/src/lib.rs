mod output;

use alumet::plugin::rust::{deserialize_config, serialize_config, AlumetPlugin};
use output::OpentelemetryOutput;
use serde::{Deserialize, Serialize};

pub struct OpentelemetryPlugin {
    config: Config,
    output: Box<OpentelemetryOutput>,
}

impl AlumetPlugin for OpentelemetryPlugin {
    fn name() -> &'static str {
        "plugin-opentelemetry"
    }

    fn version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    fn default_config() -> anyhow::Result<Option<alumet::plugin::ConfigTable>> {
        Ok(Some(serialize_config(Config::default())?))
    }

    fn init(config: alumet::plugin::ConfigTable) -> anyhow::Result<Box<Self>> {
        let config: Config = deserialize_config(config)?;
        // Create a new OpentelemetryOutput instance
        let output = Box::new(OpentelemetryOutput::new(
            config.append_unit_to_metric_name,
            config.use_unit_display_name,
            config.add_attributes_to_labels,
            config.port,
            config.host.clone(),
            config.prefix.clone(),
            config.suffix.clone(),
        )?);
        Ok(Box::new(OpentelemetryPlugin { config: config, output: output }))
    }

    fn start(&mut self, alumet: &mut alumet::plugin::AlumetPluginStart) -> anyhow::Result<()> {
        // Add output for processing measurements
        alumet.add_blocking_output(self.output.clone());
        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        self.output.meter_provider.shutdown()?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    host: String,
    prefix: String,
    suffix: String,
    port: u16,
    append_unit_to_metric_name: bool,
    use_unit_display_name: bool,
    add_attributes_to_labels: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: String::from("0.0.0.0"),
            prefix: String::from(""),
            suffix: String::from("_alumet"),
            port: 9091,
            append_unit_to_metric_name: true,
            use_unit_display_name: true,
            add_attributes_to_labels: true,
        }
    }
}
