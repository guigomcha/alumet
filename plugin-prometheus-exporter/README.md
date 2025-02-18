# Prometheus Exporter plugin

This crate is a library that defines the Prometheus Exporter plugin.

Implements a pull-based exporter which can be consumed by a Prometheus Server.

Future:
- Filter which outputs to expose
- Customize how each metric is represented in the exporter (everything is a Gauge for floats right now)
- Ensure it can be used in k8s with a nginx (or similar) to expose it in the cluster.
