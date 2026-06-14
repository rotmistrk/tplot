# node: auth
# parent: (root)
# kind: Table
# created: 2026-xx-xx 19:33:04
# last_run: 2026-xx-xx 19:33:04

sql {CREATE TABLE auth AS SELECT * FROM (VALUES ('2024-01-01 10:00:01','root','192.168.1.100','failed'), ('2024-01-01 10:00:03','admin','10.0.0.5','failed'), ('2024-01-01 10:01:15','root','192.168.1.100','failed'), ('2024-01-01 10:02:30','deploy','172.16.0.1','failed'), ('2024-01-01 10:05:00','root','192.168.1.100','failed')) AS t(ts, username, src_ip, status)}
