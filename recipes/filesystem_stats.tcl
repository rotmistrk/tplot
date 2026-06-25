# Filesystem Statistics
# Collects file sizes, modification times, and types by directory

into fs_stats --shell {find /home -maxdepth 3 -type f -printf '%s,%T@,%h,%f\n' 2>/dev/null | head -50000} --csv

# Largest directories (total size)
sql -name dir_sizes {
  SELECT column2 as directory,
         count(*) as file_count,
         sum(CAST(column0 AS BIGINT)) as total_bytes,
         round(sum(CAST(column0 AS BIGINT)) / 1048576.0, 1) as size_mb
  FROM fs_stats
  GROUP BY directory
  ORDER BY total_bytes DESC
  LIMIT 20
}

# File size distribution
sql -name size_dist {
  SELECT CASE
    WHEN CAST(column0 AS BIGINT) < 1024 THEN '<1KB'
    WHEN CAST(column0 AS BIGINT) < 10240 THEN '1-10KB'
    WHEN CAST(column0 AS BIGINT) < 102400 THEN '10-100KB'
    WHEN CAST(column0 AS BIGINT) < 1048576 THEN '100KB-1MB'
    WHEN CAST(column0 AS BIGINT) < 10485760 THEN '1-10MB'
    ELSE '>10MB'
  END as size_bucket,
  count(*) as files
  FROM fs_stats
  GROUP BY 1
  ORDER BY min(CAST(column0 AS BIGINT))
}

# Recently modified files (last 7 days)
sql -name recent_files {
  SELECT column3 as filename,
         column2 as directory,
         round(CAST(column0 AS BIGINT) / 1024.0, 1) as size_kb
  FROM fs_stats
  WHERE CAST(column1 AS DOUBLE) > extract(epoch FROM now()) - 604800
  ORDER BY CAST(column1 AS DOUBLE) DESC
  LIMIT 50
}
