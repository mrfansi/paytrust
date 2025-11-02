/// Server configuration for HTTP server
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

impl ServerConfig {
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            workers: num_cpus::get() * 2, // 2x CPU cores for I/O-bound workload
        }
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config() {
        let config = ServerConfig::new("127.0.0.1".to_string(), 8080);
        assert_eq!(config.bind_address(), "127.0.0.1:8080");
        assert!(config.workers > 0);
    }
}
