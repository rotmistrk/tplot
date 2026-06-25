# SSH Auth Failures
# Collects failed authentication attempts from auth.log/syslog
# Fields: timestamp, username, source_ip, method

into ssh_failures --shell {grep -h "Failed\|authentication failure" /var/log/auth.log /var/log/auth.log.1 2>/dev/null | sed -n 's/^\([A-Z][a-z]* [0-9 ]* [0-9:]*\).*Failed password for \(invalid user \)\?\([^ ]*\) from \([0-9.]*\).*/\1,\3,\4,password/p; s/^\([A-Z][a-z]* [0-9 ]* [0-9:]*\).*authentication failure.*user=\([^ ]*\).*rhost=\([0-9.]*\).*/\1,\2,\3,pam/p'} --csv

# Top attacking IPs
sql -name top_ips {
  SELECT source_ip, count(*) as attempts,
         count(DISTINCT username) as unique_users
  FROM ssh_failures
  GROUP BY source_ip
  ORDER BY attempts DESC
  LIMIT 20
}

# Top targeted usernames
sql -name top_users {
  SELECT username, count(*) as attempts,
         count(DISTINCT source_ip) as unique_sources
  FROM ssh_failures
  GROUP BY username
  ORDER BY attempts DESC
  LIMIT 20
}

# Attempts over time (hourly)
sql -name hourly_attempts {
  SELECT strftime(timestamp, '%Y-%m-%d %H:00') as hour,
         count(*) as attempts
  FROM ssh_failures
  GROUP BY 1
  ORDER BY 1
}
