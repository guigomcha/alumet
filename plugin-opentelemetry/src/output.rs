use alumet::{
    measurement::{MeasurementBuffer, WrappedMeasurementValue},
    pipeline::elements::{error::WriteError, output::OutputContext},
};
use anyhow::Context;
use hyper::http::StatusCode;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use opentelemetry::{
    global,
    trace::{TraceContextExt, Tracer},
    InstrumentationScope, KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_otlp::{LogExporter, MetricExporter, Protocol, SpanExporter};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::{
    logs::SdkLoggerProvider, metrics::SdkMeterProvider, trace::SdkTracerProvider,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{atomic::AtomicU64, Arc, OnceLock},
};
use tokio::runtime::Runtime;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct OpentelemetryOutput {
    scope: InstrumentationScope,
    pub meter_provider: SdkMeterProvider,
    append_unit_to_metric_name: bool,
    use_unit_display_name: bool,
    add_attributes_to_labels: bool,
    prefix: String,
    suffix: String,
}
fn get_resource() -> Resource {
    static RESOURCE: OnceLock<Resource> = OnceLock::new();
    RESOURCE
        .get_or_init(|| {
            Resource::builder()
                .with_service_name("basic-otlp-example-grpc")
                .build()
        })
        .clone()
}

fn init_metrics() -> SdkMeterProvider {
    let exporter = MetricExporter::builder()
        .with_tonic()
        .build()
        .expect("Failed to create metric exporter");

    SdkMeterProvider::builder()
        .with_periodic_exporter(exporter)
        .with_resource(get_resource())
        .build()
}

impl OpentelemetryOutput {
    pub fn new(
        append_unit_to_metric_name: bool,
        use_unit_display_name: bool,
        add_attributes_to_labels: bool,
        port: u16,
        host: String,
        prefix: String,
        suffix: String,
    ) -> anyhow::Result<OpentelemetryOutput> {
        let meter_provider = init_metrics();
        global::set_meter_provider(meter_provider.clone());
        log::info!("Created meter provider {:?}", meter_provider);
        let common_scope_attributes = vec![KeyValue::new("scope-key", "scope-value")];
        let scope = InstrumentationScope::builder("basic")
            .with_version("1.0")
            .with_attributes(common_scope_attributes)
            .build();

        Ok(Self {
            scope,
            meter_provider,
            append_unit_to_metric_name,
            use_unit_display_name,
            add_attributes_to_labels,
            prefix,
            suffix,
        })
    }
}

impl alumet::pipeline::Output for OpentelemetryOutput {

    fn write(&mut self, measurements: &MeasurementBuffer, ctx: &OutputContext) -> Result<(), WriteError> {
        if measurements.is_empty() {
            return Ok(());
        }


        for m in measurements {

            let metric = ctx.metrics.by_id(&m.metric).unwrap().clone();
            

            // Configure the name of the metric
            let full_metric = ctx
                .metrics
                .by_id(&m.metric)
                .with_context(|| format!("Unknown metric {:?}", m.metric))?;
            let metric_name = format!(
                "{}{}{}",
                self.prefix,
                sanitize_name(if self.append_unit_to_metric_name {
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
                }),
                self.suffix
            );

            // Create the default labels for all metrics and optionally add attributes
            let mut labels = vec![
                KeyValue::new("resource_kind".to_string(), m.resource.kind().to_string()),
                KeyValue::new("resource_id".to_string(), m.resource.id_string().unwrap_or_default()),
                KeyValue::new("resource_consumer_kind".to_string(), m.consumer.kind().to_string()),
                KeyValue::new(
                    "resource_consumer_id".to_string(),
                    m.consumer.id_string().unwrap_or_default(),
                ),
            ];
            if self.add_attributes_to_labels {
                // Add attributes as labels
                for (key, value) in m.attributes() {
                    let key = sanitize_name(key.to_owned());
                    labels.push(KeyValue::new(key, value.to_string()));
                }
            }
            labels.sort_by(|a, b| a.key.cmp(&b.key));


            let meter = global::meter_with_scope(self.scope.clone());

            let gauge = meter
                .f64_gauge(metric_name)
                .with_description(metric.description)
                .with_unit(metric.unit.display_name())
                .build();
            match m.value {
                WrappedMeasurementValue::F64(v) => gauge.record(v as f64, &labels),
                WrappedMeasurementValue::U64(v) => gauge.record(v as f64, &labels),
            };

        }

        Ok(())
    }
}

// Helper functions to ensure metric/label names follow Prometheus naming rules
fn sanitize_name(name: String) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}
