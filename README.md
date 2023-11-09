This project features a simple CLI that provides the ability to ingest JSON files from S3 into Redis. 

### Valid File Format
- Each line should contain valid JSON in the following format: `{"key": some valid JSON}`
- The file should be gzipped 

### Redis Data Model
Each key (string) and gzipped JSON value pair will be pushed to Redis using a SET operation (via a batch pipeline of 100 commands).

### Performance
While tested on an EC2 machine in the same VPC as Elasticache Redis (single node), this app was able to pull 13 GB of gzipped JSON from S3 and ingest it into Redis, resulting in 13 million keys, all within 10 minutes. 

## Environment Variables
- `PIPELINE_SIZE`: This parameter determines the number of SET commands to be pushed into a pipeline before execution.
- `REDIS_MODE`: Set this to `cluster` for cluster mode or `single_node` for single node mode.
- `REDIS_URL`: For cluster mode, provide a comma-separated string of cluster node URLs. For single node mode, provide a single URL, e.g. `redis://some-domain-name-001.areswo.0001.use1.cache.amazonaws.com:6379`.
- `RUST_LOG`: Set this to `info` to see the logs.
- `BUFFER_SIZE`: This variable should be set at compile time. It should correspond to the average or largest size of an ungzipped value. (See `env!("BUFFER_SIZE")` in main.rs).

## Compilation
`RUSTFLAGS="-C target-cpu=native" BUFFER_SIZE=5242880 cargo build --release`

## How To Run
It is recommended to run this CLI on an EC2 machine within the same VPC as Elasticache Redis. Since it parallelizes work using [Rayon](https://docs.rs/rayon/latest/rayon/), it's worthwhile to use a machine with a decent number of CPU cores if performance matters.
```RUST_LOG=info REDIS_MODE=single_node REDIS_URL=redis://some-domain-name-001.areswo.0001.use1.cache.amazonaws.com:6379  ./target/release/rust-redis-sink -b 'your-bucket-name' -p 'your/prefix/'```
