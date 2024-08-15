use std::sync::Mutex;
use std::sync::Arc;
use std::env;

pub struct StatusBar {
    pub time: Arc<Mutex<String>>,
    pub volume: Arc<Mutex<String>>,
    pub battery: Arc<Mutex<String>>,
    pub wifi: Arc<Mutex<String>>,
    pub time_update_period: u64,
    pub volume_update_period: u64,
    pub battery_update_period: u64,
    pub wifi_update_period: u64
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            time: Arc::new(Mutex::new(String::from("Time Loading..."))),
            volume: Arc::new(Mutex::new(String::from("Volume Loading..."))),
            battery: Arc::new(Mutex::new(String::from("Battery Loading..."))),
            wifi: Arc::new(Mutex::new(String::from("Wifi Loading..."))),
            time_update_period: env::var("TIME_UPDATE_PERIOD")
                .expect("TIME_UPDATE_PERIOD").parse().unwrap(),
            volume_update_period: env::var("VOLUME_UPDATE_PERIOD")
                .expect("VOLUME_UPDATE_PERIOD").parse().unwrap(),
            battery_update_period: env::var("BATTERY_UPDATE_PERIOD")
                .expect("BATTERY_UPDATE_PERIOD").parse().unwrap(),
            wifi_update_period: env::var("WIFI_UPDATE_PERIOD")
                .expect("WIFI_UPDATE_PERIOD").parse().unwrap()
        }
    }

    pub fn render(&self) -> String {
        let time = self.time.lock().unwrap();
        let volume = self.volume.lock().unwrap();
        let battery = self.battery.lock().unwrap();
        let wifi = self.wifi.lock().unwrap();
	    return format!(" {wifi} | {battery} | {volume} | {time} ");
    }
}
