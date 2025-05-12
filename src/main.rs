use rumqttc::{MqttOptions, AsyncClient, Event, Incoming, QoS};
use influxdb2::{Client, models::DataPoint};
use tokio::task;
use hyper::service::{make_service_fn, service_fn};
use hyper::Server;
use serde::Deserialize;
use serde_json;
use chrono::{Utc, TimeZone};
use futures::stream;
use std::sync::{Arc, Mutex};
use std::{fs, fmt};

mod request_handler;
use request_handler::RequestHandler;

#[derive(Clone, Deserialize)]
struct Config {
    mqtt_broker: String,
    mqtt_topic: String,
    mqtt_user: String,
    mqtt_password: String,
    db_url: String,
    db_org: String,
    db_bucket: String,
    db_token: String,
    db_measurement: String,
    write_password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Payload {
    #[serde(alias = "CO2")]
    co2: f64,
    #[serde(alias = "DP")]
    dp: f64,
    #[serde(alias = "H")]
    h: f64,
    #[serde(alias = "T")]
    temp: f64,
    #[serde(alias = "pCount")]
    p_count: f64,
    #[serde(default="get_timestamp")]
    time: i64,
}

impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "time: {:}, T: {:} °C, DP: {:} °C, H: {:} %, CO2: {:} ppm, pCount: {:}", Utc.timestamp_nanos(self.time), self.temp, self.dp, self.h, self.co2, self.p_count)
    }
}

fn get_timestamp() -> i64 {
    Utc::now().timestamp_nanos_opt().unwrap()
}

#[tokio::main]
async fn main() {
    let Ok(config_string) = fs::read_to_string("config.json") else {
        eprintln!("Config file not found");
        return;
    };

    let Ok(config) = serde_json::from_str::<Config>(&config_string) else {
        eprintln!("Config file not valid");
        return;
    };
    
    let db_writes_enabled: Arc<Mutex<bool>> = Arc::new(Mutex::new(true));
    let measurements = Arc::new(Mutex::new(Vec::with_capacity(10)));

    let http_handler = RequestHandler::new(db_writes_enabled.clone(), config.write_password.clone(), measurements.clone());
    let http_addr = ([0, 0, 0, 0], 8080).into();
    let http_service = make_service_fn(move |_| {
        let handler = http_handler.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                let mut handler = handler.clone();
                async move { handler.route(req).await }
            }))
        }
    });

    let http_server = Server::bind(&http_addr).serve(http_service);

    println!("Listening on http://{}", http_addr);

    task::spawn(async move {
        if let Err(e) = http_server.await {
            eprintln!("Server error: {:}", e);
        }
    });

    let mut mqtt_options = MqttOptions::new("sp-rumqtt", config.mqtt_broker.clone(), 1883);
    mqtt_options.set_credentials(config.mqtt_user.clone(), config.mqtt_password.clone());

    let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10);

    client.subscribe(config.mqtt_topic.clone(), QoS::AtMostOnce).await.unwrap();

    loop {
        match eventloop.poll().await {
            Ok(notification) => {
                let Event::Incoming(incoming) = &notification else {
                    continue;
                };

                match incoming {
                    Incoming::ConnAck(_) => println!("mqtt: Connected"),
                    Incoming::SubAck(_) => println!("mqtt: Subscribed"),
                    Incoming::Publish(publish) => {
                        println!("mqtt: Publish");

                        let Ok(payload) = serde_json::from_slice::<Payload>(&publish.payload) else {
                            eprintln!("mqtt: payload not recognized");
                            continue;
                        };

                        println!("{:}", payload);

                        let writes_enabled: bool = {
                            *db_writes_enabled.lock().unwrap()
                        };

                        let mut m = measurements.lock().unwrap();
                        if m.len() == 10 {
                            m.remove(0);
                        }
                        m.push(payload.clone());

                        if writes_enabled {
                            let config_clone = config.clone();
                            task::spawn(async move {
                                let influxdb_client = Client::new(config_clone.db_url, config_clone.db_org, config_clone.db_token);

                                let data_point = vec![DataPoint::builder(config_clone.db_measurement)
                                    .field("T", payload.temp)
                                    .field("H", payload.h)
                                    .field("DP", payload.dp)
                                    .field("CO2", payload.co2)
                                    .field("pCount", payload.p_count)
                                    .timestamp(payload.time)
//                                    .timestamp(Utc::now().timestamp_nanos_opt().unwrap())
                                    .build()
                                    .unwrap()];

                                let write_result = influxdb_client.write(&config_clone.db_bucket, stream::iter(data_point)).await;
                                match write_result {
                                    Ok(_) => println!("Database write ok!"),
                                    Err(e) => eprintln!("Database write failed: {:}", e),
                                }
                            });
                        }
                    },
                    _ => (),
                }
            },
            Err(e) => println!("mqtt: {e}"),
        }
    }
}

