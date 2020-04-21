use dotenv::dotenv;
use std::env;
use reqwest;
use serde::Deserialize;
use std::collections::HashMap;
use warp::Filter;
use std::convert::Infallible;
use tokio::sync::{Mutex};
use std::sync::{Arc};

pub type Stations = Vec<Station>;

#[derive(Deserialize, Debug)]
pub struct Station {
    #[serde(rename = "Station")]
    station: String,
    #[serde(rename = "Distance")]
    distance: String,
    #[serde(rename = "Petrol")]
    petrol: bool,
    #[serde(rename = "Diesel")]
    diesel: bool,
    #[serde(rename = "PetrolPrice")]
    petrol_price: f64,
    #[serde(rename = "DieselPrice")]
    diesel_price: f64,
}

type Prices = Arc<Mutex<HashMap<String, Stations>>>;

async fn get_prices(postcode: &String, radius: &u32) -> Result<Stations, reqwest::Error> {
    let params = [
        ("location", postcode),
        ("radius", &radius.to_string())
    ];

    let client = reqwest::Client::new();

    let body = client.get("https://www.allstarcard.co.uk/Umbraco/Api/Fuelcards/GetNearestStations")
        .query(&params)
        .send()
        .await?
        .json::<Stations>()
        .await?;

    Ok(body)
}

async fn render(prices: Prices) -> Result<impl warp::Reply, Infallible> {
    let mut resp: Vec<String> = Vec::new();
    
    resp.push("# HELP fuel_price Fuel price".to_string());
    resp.push("# TYPE fuel_price gauge".to_string());

    for t in prices.lock().await.iter() {
        let postcode = t.0;
        let data = t.1;

        for station in data {
            if station.petrol {
                resp.push(
                    format!(
                        "fuel_price{{postcode=\"{}\", type=\"petrol\", provider=\"{}\", distance=\"{}\" }} {}",
                        postcode,
                        station.station,
                        station.distance,
                        station.petrol_price / f64::from(10)
                    )
                );
            }

            if station.diesel {
                resp.push(
                    format!(
                        "fuel_price{{postcode=\"{}\", type=\"diesel\", provider=\"{}\", distance=\"{}\" }} {}",
                        postcode,
                        station.station,
                        station.distance,
                        station.diesel_price / f64::from(10)
                    )
                );
            }
        }
    }

    Ok(format!("{}", resp.join("\n")))
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let port: u16 = env::var("PORT").unwrap().parse().unwrap();
    let postcodes: Vec<_> = env::var("POSTCODES").unwrap().split(",").map(|s| s.to_string()).collect();
    let radius: u32 = env::var("RADIUS").unwrap().parse().unwrap();

    let prices: Prices = Arc::new(Mutex::new(HashMap::new()));

    for postcode in postcodes {
        let result = get_prices(&postcode, &radius)
            .await
            .unwrap();

        println!("Loaded {} stations for {}", result.len(), postcode);
        prices.lock().await.insert(postcode, result);
    }

    let filter = warp::any().map(move || prices.clone());

    let metrics = warp::path("metrics")
        .and(filter)
        .and_then(render);

    println!("Running on 0.0.0.0:{}/metrics", port);

    warp::serve(metrics)
        .run(([0, 0, 0, 0], port))
        .await;
}
