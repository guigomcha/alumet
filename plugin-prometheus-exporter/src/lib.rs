mod output;

use alumet::plugin::rust::{deserialize_config, serialize_config, AlumetPlugin};
use output::create_prometheus_instance;
use serde::{Deserialize, Serialize};

pub struct PrometheusPlugin {
    config: Option<Config>,
}

impl AlumetPlugin for PrometheusPlugin {
    fn name() -> &'static str {
        "plugin-prometheus"
    }

    fn version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    fn default_config() -> anyhow::Result<Option<alumet::plugin::ConfigTable>> {
        Ok(Some(serialize_config(Config::default())?))
    }

    fn init(config: alumet::plugin::ConfigTable) -> anyhow::Result<Box<Self>> {
        let config = deserialize_config(config)?;
        Ok(Box::new(PrometheusPlugin { config: Some(config) }))
    }

    fn start(&mut self, alumet: &mut alumet::plugin::AlumetPluginStart) -> anyhow::Result<()> {
        let config = self.config.take().unwrap();

        // Create a new PrometheusOutput instance
        let prometheus_output = create_prometheus_instance(config.host, config.port)?;

        // Add output for processing measurements
        alumet.add_blocking_output(Box::new(prometheus_output));

        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    host: String,
    port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: String::from("127.0.0.1"),
            port: 9091,
        }
    }
}