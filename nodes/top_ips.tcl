# node: top_ips
# parent: ufw
# icon: [Q]
# rows: 15

sql -name top_ips {SELECT src_ip, count(*) as hits FROM ufw GROUP BY src_ip ORDER BY hits DESC LIMIT 15}
