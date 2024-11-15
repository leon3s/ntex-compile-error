use ntex::web;
use ntex_cors::Cors;

use nanocl_utils::ntex::middlewares;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod, SslVerifyMode};

async fn unhandled() -> web::HttpResponse {
    web::HttpResponse::NotFound().body("Not Found")
}

#[derive(Clone)]
pub struct SslConfig {
  pub cert: Option<String>,
  pub cert_key: Option<String>,
  pub cert_ca: Option<String>,
}

#[derive(Clone)]
pub struct DaemonConfig {
  pub hosts: Vec<String>,
  pub ssl: Option<SslConfig>,
}

#[derive(Clone)]
pub struct SystemStateInner {
  config: DaemonConfig,
}

#[derive(Clone)]
pub struct SystemState {
  inner: SystemStateInner
}

pub async fn gen(
  daemon_state: SystemState,
) -> std::io::Result<ntex::server::Server> {
  let daemon_state_ptr = daemon_state.clone();
  let mut server = web::HttpServer::new(move || {
    web::App::new()
      // bind config state
      .state(daemon_state_ptr.clone())
      .state(
        web::types::PayloadConfig::new(20_000_000_000), // <- limit size of the payload
      )
      .wrap(Cors::new().finish())
      .wrap(middlewares::Versioning::new("1.0").finish())
      .wrap(middlewares::SerializeError)
      // Default logger middleware
      .wrap(web::middleware::Logger::default())
      // Set Json body max size
      .state(web::types::JsonConfig::default().limit(20_000_000))
      // .configure(services::ntex_config)
      .default_service(web::route().to(unhandled))
  });
  let config = daemon_state.inner.config.clone();
  let mut count = 0;
  let hosts = config.hosts.clone();
  let len = hosts.len();
  while count < len {
    let host = &hosts[count];
    if host.starts_with("unix://") {
      let addr = host.replace("unix://", "");
      server = match server.bind_uds(&addr) {
        Err(err) => {
          return Err(err);
        }
        Ok(server) => server,
      };
    } else if host.starts_with("tcp://") {
      let addr = host.replace("tcp://", "");
      if let Some(ssl) = config.ssl.clone() {
        let cert = ssl.cert.clone().unwrap();
        let cert_key = ssl.cert_key.clone().unwrap();
        let cert_ca = ssl.cert_ca.clone().unwrap();
        server = match server.bind_openssl(&addr, {
          let mut builder =
            SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
          builder
            .set_private_key_file(cert_key, SslFiletype::PEM)
            .unwrap();
          builder.set_certificate_chain_file(cert).unwrap();
          builder.set_ca_file(cert_ca).expect("Failed to set ca file");
          builder.check_private_key().unwrap();
          builder.set_verify(
            SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT,
          );
          builder
        }) {
          Err(err) => {
            return Err(err);
          }
          Ok(server) => server,
        };
      } else {
        server = match server.bind(&addr) {
          Err(err) => {
            return Err(err);
          }
          Ok(server) => server,
        };
      }
    } else {
      return Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "invalid protocol [tcp:// | unix://] allowed",
      ));
    }
    count += 1;
  }
  server = server.workers(num_cpus::get());
  Ok(server.run())
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let daemon_state = SystemState {
        inner: SystemStateInner {
            config: DaemonConfig {
                hosts: vec!["tcp://0.0.0.0:8080".to_string()],
                ssl: None,
            },
        },
    };
    let server = gen(daemon_state).await?;
    server.await?;
    Ok(())
}
