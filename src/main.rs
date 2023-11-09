use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use log::{error, info};
use std::io::Read;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Instant;
mod redis_wrappers;
use redis_wrappers::{RedisConnection, RedisPipeline, REDIS_CLIENT};
mod aws_wrappers;
use aws_wrappers::{get_s3_keys, get_s3_object};
use rayon::prelude::*;
use structopt::StructOpt;
use tokio::io::AsyncReadExt;
use tokio::runtime::Runtime;

fn ingest_to_redis_from_s3(
    s3_bucket: &str,
    s3_prefix: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // we use tokio runtime to run async functions in a syncronious way
    let rt = Runtime::new().unwrap();

    let s3_keys =
        rt.block_on(async { get_s3_keys(s3_bucket.to_string(), s3_prefix.to_string()).await });
    info!(
        "Total number of files to ingest into Redis: {:?}",
        s3_keys.len()
    );

    let ingestion_start = Instant::now();
    let total_keys_counter: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));

    // we're using rayon (par_iter) to parallelize the ingestion of files on S3 into Redis
    s3_keys.par_iter().for_each(|key| {
        let mut con = REDIS_CLIENT.get_connection();
        let mut pipe = match con {
            RedisConnection::Single(ref mut _con) => RedisPipeline::Single(redis::pipe()),
            RedisConnection::Cluster(ref mut _con) => {
                RedisPipeline::Cluster(redis::cluster::cluster_pipe())
            }
        };
        let buffer_string: String = rt.block_on(async {
            let object = get_s3_object(s3_bucket.to_string(), key.clone()).await;
            let mut body = object.body.into_async_read();
            let mut buffer = Vec::new();
            body.read_to_end(&mut buffer).await.unwrap();
            let mut decoder = GzDecoder::new(&buffer[..]);
            let mut buffer_string =
                String::with_capacity(env!("BUFFER_SIZE").parse::<usize>().unwrap());
            decoder.read_to_string(&mut buffer_string).unwrap();
            buffer_string
        });

        let mut counter: usize = 0;

        for line in buffer_string.lines() {
            let item: serde_json::Value = serde_json::from_str(line).unwrap();
            if let Some(obj) = item.as_object() {
                let (obj_key, value) = obj.iter().next().unwrap();
                let value_str = value.to_string();
                let mut encoder = GzEncoder::new(
                    Vec::with_capacity(value_str.len() / 20),
                    Compression::best(),
                );
                encoder.write_all(value_str.as_bytes()).unwrap();
                let compressed_bytes = encoder.finish().unwrap();

                match &mut pipe {
                    RedisPipeline::Single(ref mut pipe) => {
                        pipe.cmd("SET").arg(obj_key).arg(compressed_bytes).ignore();
                    }
                    RedisPipeline::Cluster(ref mut pipe) => {
                        pipe.cmd("SET").arg(obj_key).arg(compressed_bytes).ignore();
                    }
                }
                counter += 1;

                if counter >= 100 {
                    match (&mut pipe, &mut con) {
                        (RedisPipeline::Single(pipe), RedisConnection::Single(ref mut con)) => {
                            pipe.execute(con);
                            pipe.clear();
                        }
                        (RedisPipeline::Cluster(pipe), RedisConnection::Cluster(ref mut con)) => {
                            pipe.execute(con);
                            pipe.clear();
                        }
                        _ => {
                            panic!("Mismatch between pipeline and connection types")
                        }
                    }
                    counter = 0;
                    *total_keys_counter.lock().unwrap() += 100;
                }
            }
        }
        // Execute any remaining commands in the pipeline
        if counter > 0 {
            match (&mut pipe, &mut con) {
                (RedisPipeline::Single(pipe), RedisConnection::Single(ref mut con)) => {
                    pipe.execute(con);
                    pipe.clear();
                }
                (RedisPipeline::Cluster(pipe), RedisConnection::Cluster(ref mut con)) => {
                    pipe.execute(con);
                    pipe.clear();
                }
                _ => {
                    panic!("Mismatch between pipeline and connection types")
                }
            }
            info!("The file {} was ingested into Redis", key);
        }
    });

    info!(
        "total number of keys ingested: {}",
        *total_keys_counter.lock().unwrap()
    );

    info!(
        "time to ingest to Redis from S3: {:?}",
        ingestion_start.elapsed()
    );

    Ok("Ingestion to Redis from S3 completed".to_string())
}

#[derive(StructOpt)]
struct Cli {
    #[structopt(short = "b", long = "bucket", about = "Sets the S3 bucket")]
    s3_bucket: String,

    #[structopt(short = "p", long = "prefix", about = "Sets the S3 prefix")]
    s3_prefix: String,
}

fn main() {
    env_logger::init();

    let args = Cli::from_args();
    let s3_bucket = &args.s3_bucket;
    let s3_prefix = &args.s3_prefix;

    match ingest_to_redis_from_s3(s3_bucket, s3_prefix) {
        Ok(result) => info!("{}", result),
        Err(err) => error!("Error: {}", err),
    }
}
