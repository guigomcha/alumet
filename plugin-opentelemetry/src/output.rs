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
        .with_http()
        .with_protocol(Protocol::HttpBinary) //can be changed to `Protocol::HttpJson` to export in JSON format
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
            let meter = global::meter_with_scope(self.scope.clone());

            let counter = meter
                .u64_counter("test_counter")
                .with_description("a simple counter for demo purposes.")
                .with_unit("my_unit")
                .build();
            for _ in 0..10 {
                counter.add(1, &[KeyValue::new("test_key", "test_value")]);
            }
            counter.add(1, &[KeyValue::new("test_key", "test_value")]);
        }

        Ok(())
    }
}

