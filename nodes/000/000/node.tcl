# node: by_user
# parent: auth
# kind: Query
# created: 2026-xx-xx 00:29:15
# last_run: 2026-xx-xx 00:29:15
# rows: 3

sql -name by_user {SELECT username, count(*) as attempts FROM auth GROUP BY username ORDER BY attempts DESC}
