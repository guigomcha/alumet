# Post Processing Plugin

Plugin which can be used as last step in the plugin chain to create a series of labels that can be used by output plugins:

- display_name: Creates a human-friendly display name for all metrics.
  append_unit_to_metric_name = true
  use_unit_display_name = true
  prefix = "alumet_"
  suffix = ""
- output: Creates a label to indicate if the metric should be consumed by the output
  filter_discard: [
    "regex"
  ]
  filter_accept: [
    "regex"
  ]