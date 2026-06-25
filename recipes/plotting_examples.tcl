# Plotting Examples
# Prerequisites: run one of the data collection recipes first

# Bar chart — top IPs by SSH failure count
# (requires ssh_auth_failures recipe first)
plot bar top_ips source_ip attempts

# Bar chart — file size distribution
# (requires filesystem_stats recipe first)
plot bar size_dist size_bucket files

# Line chart — SSH attempts over time
# (requires ssh_auth_failures recipe first)
plot line hourly_attempts hour attempts

# Bar chart — network connection states
# (requires network_connections recipe first)
plot bar conn_states state count
