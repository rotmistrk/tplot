# System Log Events
# Collects kernel/system events from syslog with severity parsing

into syslog_events --shell {journalctl --no-pager -o short-iso -n 5000 2>/dev/null || tail -5000 /var/log/syslog 2>/dev/null || tail -5000 /var/log/messages} --csv

# Events by process/unit
sql -name log_by_source {
  SELECT split_part(column2, '[', 1) as source,
         count(*) as events
  FROM syslog_events
  GROUP BY 1
  ORDER BY events DESC
  LIMIT 20
}

# Error/warning events
sql -name log_errors {
  SELECT *
  FROM syslog_events
  WHERE lower(column3) LIKE '%error%'
     OR lower(column3) LIKE '%fail%'
     OR lower(column3) LIKE '%crit%'
  ORDER BY column0 DESC
  LIMIT 100
}
