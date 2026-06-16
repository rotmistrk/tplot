# node: ufw
# icon: [T]

sql {    CREATE OR REPLACE TABLE ufw AS    SELECT        regexp_extract(column0, 'SRC=([0-9.]+)', 1) as src_ip,        TRY_CAST(regexp_extract(column0, 'DPT=([0-9]+)', 1) AS INTEGER) as dst_port,        regexp_extract(column0, 'PROTO=(\w+)', 1) as proto    FROM read_csv('/var/log/ufw.log', header=false, sep=E'\x01')    WHERE column0 LIKE '%UFW BLOCK%'}
