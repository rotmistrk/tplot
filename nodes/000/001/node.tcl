# node: by_ip
# parent: auth
# kind: Query
# created: 2026-xx-xx 00:29:21
# last_run: 2026-xx-xx 00:29:21
# rows: 3

sql -name by_ip {SELECT src_ip, count(*) as attempts FROM auth GROUP BY src_ip ORDER BY attempts DESC}
