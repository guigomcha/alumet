use alumet::{
    measurement::{MeasurementBuffer, WrappedMeasurementValue},
    pipeline::{
        elements::{error::WriteError, output::OutputContext},
        Output,
    },
    plugin::rust::AlumetPlugin,
};
use anyhow::Context;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use parking_lot::RwLock;
use prometheus_client::{
    encoding::text::encode,
    metrics::{family::Family, gauge::Gauge},
    registry::Registry,
};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::runtime::Runtime;

#[derive(Clone)]
pub struct MetricState {
    registry: Arc<RwLock<Registry>>,
    metrics: Arc<RwLock<HashMap<String, Family<Vec<(String, String)>, Gauge>>>>,
}

pub struct PrometheusOutput {
    state: MetricState,
}

impl alumet::pipeline::Output for PrometheusOutput {
    fn write(&mut self, measurements: &MeasurementBuffer, ctx: &OutputContext) -> Result<(), WriteError> {
        if measurements.is_empty() {
            return Ok(());
        }

        let mut metrics = self.state.metrics.write();
        let mut registry = self.state.registry.write();

        for m in measurements {
            let metric = ctx.metrics.by_id(&m.metric).unwrap();
            let metric_name = sanitize_metric_name(&metric.name);

            // Create labels/tags as Vec of tuples for proper label ordering
            let mut labels = vec![
                ("resource_kind".to_string(), m.resource.kind().to_string()),
                ("resource_id".to_string(), m.resource.id_string().unwrap_or_default()),
                ("resource_consumer_kind".to_string(), m.consumer.kind().to_string()),
                ("resource_consumer_id".to_string(), m.consumer.id_string().unwrap_or_default()),
            ];

            // Add attributes as labels
            for (key, value) in m.attributes() {
                let key = sanitize_label_name(key);
                labels.push((key, value.to_string()));
            }

            // Sort labels for consistent ordering
            labels.sort_by(|a, b| a.0.cmp(&b.0));

            // Get or create metric family with proper error handling
            let family = if let Some(family) = metrics.get(&metric_name) {
                family
            } else {
                let family = Family::default();
                
                // Just register the metric - if it panics, the mutex guard will be dropped properly
                registry.register(
                    metric_name.clone(),
                    &metric.description,
                    family.clone(),
                );
                
                metrics.insert(metric_name.clone(), family.clone());
                metrics.get(&metric_name)
                    .ok_or_else(|| WriteError::Fatal(
                        anyhow::anyhow!("Failed to retrieve metric after registration")
                    ))?
            };

            // Update metric value
            let gauge = family.get_or_create(&labels);
            match m.value {
                WrappedMeasurementValue::F64(v) => gauge.set(v as i64),
                WrappedMeasurementValue::U64(v) => gauge.set(v as i64),
            };
        }

        Ok(())
    }
}

// Helper functions to ensure metric/label names follow Prometheus naming rules
fn sanitize_metric_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

fn sanitize_label_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

// New function to create a new instance of PrometheusOutput
pub fn create_prometheus_instance(host: String, port: u16) -> anyhow::Result<PrometheusOutput> {
    // Create metric state
    let registry = Arc::new(RwLock::new(Registry::default()));
    let metrics = Arc::new(RwLock::new(HashMap::new()));
    let state = MetricState { registry, metrics };

    // Start HTTP server
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .context("Invalid host:port configuration")?;

    let state_clone = state.clone();

    // Create a new Tokio runtime for the HTTP server
    let rt = Runtime::new().context("Failed to create Tokio runtime")?;

    // Spawn the server on the runtime
    rt.spawn(async move {
        let make_svc = make_service_fn(move |_conn| {
            let state = state_clone.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                    let state = state.clone();
                    async move {
                        if req.uri().path() != "/metrics" {
                            return Ok::<Response<Body>, hyper::Error>(Response::builder()
                                .status(404)
                                .body(Body::from("Not Found"))
                                .unwrap());
                        }

                        let mut buf = String::new();
                        if let Err(e) = encode(&mut buf, &*state.registry.read()) {
                            log::error!("Failed to encode metrics: {}", e);
                            return Ok(Response::builder()
                                .status(500)
                                .body(Body::from("Internal Server Error"))
                                .unwrap());
                        }

                        Ok(Response::builder()
                            .header("Content-Type", "application/openmetrics-text; version=1.0.0; charset=utf-8")
                            .body(Body::from(buf))
                            .unwrap())
                    }
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);
        log::info!("Prometheus metrics server listening on http://{}/metrics", addr);

        if let Err(e) = server.await {
            log::error!("Prometheus server error: {}", e);
        }
    });

    // Keep runtime alive
    std::thread::spawn(move || {
        rt.block_on(async {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        });
    });

    Ok(PrometheusOutput { state })
}