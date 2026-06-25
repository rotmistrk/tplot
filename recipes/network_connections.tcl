# Network Connections
# Collects current network connections and listening ports
# Uses ss (socket statistics) for modern systems

into net_conns --shell {ss -tunapH 2>/dev/null | awk '{print $1","$2","$5","$6","$7}' | sed 's/users:(("/,/; s/".*//'} --csv

# Connections by state
sql -name conn_states {
  SELECT column0 as state, count(*) as count
  FROM net_conns
  GROUP BY state
  ORDER BY count DESC
}

# Top remote IPs (established connections)
sql -name remote_ips {
  SELECT split_part(column3, ':', 1) as remote_ip,
         count(*) as connections
  FROM net_conns
  WHERE column0 = 'ESTAB'
  GROUP BY 1
  ORDER BY connections DESC
  LIMIT 20
}

# Listening ports
sql -name listening {
  SELECT column3 as local_addr,
         column5 as process
  FROM net_conns
  WHERE column0 = 'LISTEN'
  ORDER BY local_addr
}
