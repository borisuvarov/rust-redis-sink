use lazy_static::lazy_static;
use redis::cluster::{
    ClusterClient as RedisClusterClient, ClusterConnection as RedisClusterConnection,
    ClusterPipeline,
};
use redis::{
    Client as RedisSingleNodeClient, Connection as RedisSingleNodeConnection,
    Pipeline as SingleNodePipeline,
};
use std::env;

pub enum RedisClientUnion {
    Single(RedisSingleNodeClient),
    Cluster(RedisClusterClient),
}

pub struct RedisClient {
    pub client: RedisClientUnion,
}

pub enum RedisConnection {
    Single(RedisSingleNodeConnection),
    Cluster(RedisClusterConnection),
}

pub enum RedisPipeline {
    Single(SingleNodePipeline),
    Cluster(ClusterPipeline),
}

// syncronious implementation: since we're using pipelines in threads, it doesn't make much sense to complicate things
impl RedisClient {
    pub fn new() -> Self {
        let url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

        let client = match env::var("REDIS_MODE") {
            Ok(mode) if mode == "cluster" => {
                let nodes = url.split(',').collect();
                let client =
                    RedisClusterClient::new(nodes).expect("Failed to create Redis Cluster client");
                RedisClientUnion::Cluster(client)
            }
            _ => {
                let client: RedisSingleNodeClient = RedisSingleNodeClient::open(url.as_str())
                    .expect("Failed to create Redis client");
                RedisClientUnion::Single(client)
            }
        };

        RedisClient { client }
    }

    pub fn get_connection(&self) -> RedisConnection {
        match &self.client {
            RedisClientUnion::Single(client) => {
                let conn = client
                    .get_connection()
                    .expect("Failed to get Redis connection");
                RedisConnection::Single(conn)
            }
            RedisClientUnion::Cluster(client) => {
                let conn = client
                    .get_connection()
                    .expect("Failed to get Redis connection");
                RedisConnection::Cluster(conn)
            }
        }
    }
}

lazy_static! {
    pub static ref REDIS_CLIENT: RedisClient = RedisClient::new();
}
