<style>
pre {
  white-space: pre-wrap;
  word-wrap: break-word;
}
</style>

# List files

## CLI
### Help
#### Command
| Command usage | Description                                                |
| ------------- | ---------------------------------------------------------- |
| `lakestream ls <uri>` | List objects on Local Filesystem or an S3 bucket.     |

#### Arguments
| Argument | Description |
| -------- | ----------- |
| `<uri>`  | URI to list objects from. E.g. s3://bucket-name/     |

#### Options
| Option                 | Description                                                 |
| ---------------------- | ------------------------------------------------------------ |
| `-n`, `--name <name>` | Filter objects based on name. E.g. 'foo', 'foo.*', '.*bar'   |
| `-s`, `--size <size>` | Filter objects based on size. E.g. '-1K', '+4M', '+1G', '-1G', '5G', '1G-2G' |
| `-t`, `--mtime <mtime>` | Filter objects based on the time offset. E.g. '-60s', '+5m', '-1h', '+2D', '-3W', '+1M', '-1Y' |
| `-r`, `--recursive`   | List (virtual) subdirectories recursively                   |
| `-m`, `--max-files <max_files>` | Maximum number of files to list [default: 1000]  |
| `-h`, `--help`        | Print help                                                  |

### Examples

#### Local Filesystem
```
# Find all files in the current directory, larger than 100 MB and modified
# within the last 2 days.
lakestream ls . --size "+100M" --mtime "-2D"

# Find all files in the "log_files" directory, with names ending in ".log" and
# modified within the last week.
lakestream ls log_files/ --name ".*.log$" --mtime "-1W"

# List all files in the current directory and its subdirectories (recursively),
# larger than 1 GB.
lakestream ls . --size "+1G" --recursive

# Find all files in the "images" directory, with names ending in ".jpg" and
# sizes between 1 MB and 5 MB.
lakestream ls images/ --name ".*.jpg$" --size "1M-5M"

# Find all files in the "documents" directory and its subdirectories (recursively),
# with names containing "report" and modified within the last 1 month.
lakestream ls documents/ --name "report" --mtime "-1M" --recursive

# Find all files with the ".json" extension, between 10 KB and 1 MB, modified
# within the last 2 weeks in the "configs" directory
lakestream ls configs/ --name ".json$" --size "10K-1M" --mtime "-2W"

# Find all files with the ".log" extension, larger than 5 MB, and modified
# within the last 1 day, 8 hours, and 20 minutes in the "logs" directory
lakestream ls logs/ --name ".log$" --size "+5M" --mtime "-1D8h20m"

# Find all files containing "report" and ending with ".csv", between 50 KB and 2 MB,
# and modified within the last 2 days, 6 hours, and 30 minutes in the "reports" directory
lakestream ls reports/ --name "report*.csv$" --size "50K-2M" --mtime "-2D6h30m"

# Find all .log files smaller than 500 KB and modified more than 1 month ago in
# the "logs" directory, recursively
lakestream ls logs/ --name "*.log" --size "-500K" --mtime "+1M" --recursive
```

#### S3 Bucket
```
# Find all files in the "videos" directory of an S3 bucket, modified within
# the last 3 days, 8 hours, and 20 minutes.
lakestream ls s3://bucket-name/videos/ --mtime "-3D8h20m"

# Find all files in an S3 bucket, with names containing "backup" and
# sizes between 500 MB and 2 GB.
lakestream ls s3://bucket-name/ --name "backup" --size "500M-2G"

# List the first 50 files in an S3 bucket.
lakestream ls s3://bucket-name/ --max-files 50

# Find all files in the "reports" directory, with names containing "2023"
# and modified within the last 30 days, in a given S3 bucket.
lakestream ls s3://bucket-name/reports/ --name "2023" --mtime "-30D"

# Find all .txt files larger than 1 MB modified within the last 7 days and limit
# the result to 20 files
lakestream ls s3://bucket-name/ --name ".txt$" --size "+1M" --mtime "-7D" --max-files 20

# List the first 100 files in an S3 bucket of size larger than 1K.
lakestream ls s3://bucket-name/ --max-files 100 --size "+1K"

# Find all files containing "backup" and ending with ".zip", larger than 100 MB,
# and modified more than 6 months ago, recursively
lakestream ls s3://bucket-name/ --name "backup.zip$" --size "+100M" --mtime "+6M" --recursive

# Find all files with the ".txt" extension, smaller than 1 MB, and modified
# within the last 3 hours in the "texts" directory
lakestream ls s3://bucket-name/texts/ --name "*.txt" --size "-1M" --mtime "-3h"

# Find all .mp4 files larger than 5 GB modified more than 3 months ago,
# and limit the result to 100 files
lakestream ls s3://bucket-name/ --name "*.mp4" --size "+5G" --mtime "+3M" --max-files 100
```



