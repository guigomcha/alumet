# Post Processing Plugin

Plugin which can be used as last step in the plugin chain to create a series of attributes that can be used by output plugins:

- display_name: Creates a human-friendly display name for all metrics as a attribute. The config.toml should look like:

    ```toml
    [plugins.post-processing]
    append_unit_to_metric_name = true
    use_unit_display_name = true
    prefix = ""
    suffix = ""  
    ```
